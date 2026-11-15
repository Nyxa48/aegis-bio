use serde::{Deserialize, Serialize};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Online,
    Offline,
}

impl ConnectionState {
    pub fn label(self) -> &'static str {
        match self {
            ConnectionState::Online => "ONLINE",
            ConnectionState::Offline => "OFFLINE",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SyncBatch {
    pub telemetry_seq_ids: Vec<i64>,
    pub crew_input_ids: Vec<i64>,
    pub sleep_epoch_ids: Vec<i64>,
    pub daily_state_key: Option<(String, i64)>,
}

#[derive(Debug, Clone)]
pub struct AckEvent {
    pub delivered: bool,
    pub packet_type: String,
    pub sync_batch: Option<SyncBatch>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundFrame {
    pub crew_id: String,
    pub packet_type: String,
    pub payload: serde_json::Value,
    #[serde(skip)]
    pub sync_batch: Option<SyncBatch>,
}

impl OutboundFrame {
    pub fn to_json_line(&self) -> serde_json::Result<String> {
        let mut line = serde_json::to_string(self)?;
        line.push('\n');
        Ok(line)
    }
}

enum NetCommand {
    Send(OutboundFrame),
    Probe,
}

pub struct BiomonLanClient {
    target_hub_addr: String,
    state: Arc<Mutex<ConnectionState>>,
    manual_blackout: Arc<AtomicBool>,
    hub_reachable: Arc<AtomicBool>,
    frames_sent: Arc<AtomicUsize>,
    cmd_tx: mpsc::UnboundedSender<NetCommand>,
    ack_rx: Mutex<mpsc::UnboundedReceiver<AckEvent>>,
    _runtime: tokio::runtime::Runtime,
}

async fn try_connect(addr: &str) -> Option<TcpStream> {
    tokio::time::timeout(Duration::from_millis(350), TcpStream::connect(addr))
        .await
        .ok()
        .and_then(|res| res.ok())
}

fn update_conn_state(
    state: &Arc<Mutex<ConnectionState>>,
    hub_reachable: &Arc<AtomicBool>,
    manual_blackout: &Arc<AtomicBool>,
    reachable: bool,
) {
    hub_reachable.store(reachable, Ordering::Relaxed);
    let next = if !manual_blackout.load(Ordering::Relaxed) && reachable {
        ConnectionState::Online
    } else {
        ConnectionState::Offline
    };
    if let Ok(mut guard) = state.lock() {
        *guard = next;
    }
}

impl BiomonLanClient {
    pub fn new(target_hub_addr: impl Into<String>) -> Self {
        let target_hub_addr = target_hub_addr.into();
        let state = Arc::new(Mutex::new(ConnectionState::Offline));
        let manual_blackout = Arc::new(AtomicBool::new(false));
        let hub_reachable = Arc::new(AtomicBool::new(false));
        let frames_sent = Arc::new(AtomicUsize::new(0));
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<NetCommand>();
        let (ack_tx, ack_rx) = mpsc::unbounded_channel::<AckEvent>();

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("failed to build BiomonLanClient tokio runtime");

        let addr_clone = target_hub_addr.clone();
        let state_clone = Arc::clone(&state);
        let blackout_clone = Arc::clone(&manual_blackout);
        let reachable_clone = Arc::clone(&hub_reachable);
        let sent_clone = Arc::clone(&frames_sent);

        // Kick off an immediate probe on startup so UI quickly shows live hub state
        let _ = cmd_tx.send(NetCommand::Probe);

        runtime.spawn(async move {
            let mut stream: Option<TcpStream> = None;
            let mut probe_interval = tokio::time::interval(Duration::from_millis(1500));

            loop {
                tokio::select! {
                    maybe_cmd = cmd_rx.recv() => {
                        let Some(cmd) = maybe_cmd else { break; };
                        match cmd {
                            NetCommand::Probe => {
                                if blackout_clone.load(Ordering::Relaxed) {
                                    stream = None;
                                    update_conn_state(&state_clone, &reachable_clone, &blackout_clone, false);
                                    continue;
                                }
                                if let Some(s) = stream.as_ref() {
                                    let mut buf = [0u8; 1];
                                    match s.try_read(&mut buf) {
                                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                            update_conn_state(&state_clone, &reachable_clone, &blackout_clone, true);
                                        }
                                        _ => {
                                            stream = None;
                                        }
                                    }
                                }
                                if stream.is_none() {
                                    if let Some(s) = try_connect(&addr_clone).await {
                                        stream = Some(s);
                                        update_conn_state(&state_clone, &reachable_clone, &blackout_clone, true);
                                    } else {
                                        update_conn_state(&state_clone, &reachable_clone, &blackout_clone, false);
                                    }
                                }
                            }
                            NetCommand::Send(frame) => {
                                if blackout_clone.load(Ordering::Relaxed) {
                                    stream = None;
                                    update_conn_state(&state_clone, &reachable_clone, &blackout_clone, false);
                                    let _ = ack_tx.send(AckEvent {
                                        delivered: false,
                                        packet_type: frame.packet_type,
                                        sync_batch: frame.sync_batch,
                                        error: Some("MANUAL BLACKOUT".to_string()),
                                    });
                                    continue;
                                }

                                let Ok(line) = frame.to_json_line() else {
                                    let _ = ack_tx.send(AckEvent {
                                        delivered: false,
                                        packet_type: frame.packet_type,
                                        sync_batch: frame.sync_batch,
                                        error: Some("SERIALIZATION ERROR".to_string()),
                                    });
                                    continue;
                                };

                                if stream.is_none() {
                                    stream = try_connect(&addr_clone).await;
                                }

                                let mut delivered = false;
                                if let Some(s) = stream.as_mut() {
                                    if s.write_all(line.as_bytes()).await.is_ok() && s.flush().await.is_ok() {
                                        delivered = true;
                                    } else {
                                        // Stale socket: drop and retry once on a fresh TCP stream
                                        stream = try_connect(&addr_clone).await;
                                        if let Some(fresh) = stream.as_mut()
                                            && fresh.write_all(line.as_bytes()).await.is_ok()
                                            && fresh.flush().await.is_ok()
                                        {
                                            delivered = true;
                                        } else {
                                            stream = None;
                                        }
                                    }
                                }

                                if delivered {
                                    update_conn_state(&state_clone, &reachable_clone, &blackout_clone, true);
                                    sent_clone.fetch_add(1, Ordering::Relaxed);
                                    let _ = ack_tx.send(AckEvent {
                                        delivered: true,
                                        packet_type: frame.packet_type,
                                        sync_batch: frame.sync_batch,
                                        error: None,
                                    });
                                } else {
                                    update_conn_state(&state_clone, &reachable_clone, &blackout_clone, false);
                                    let _ = ack_tx.send(AckEvent {
                                        delivered: false,
                                        packet_type: frame.packet_type,
                                        sync_batch: frame.sync_batch,
                                        error: Some("HUB UNREACHABLE".to_string()),
                                    });
                                }
                            }
                        }
                    }
                    _ = probe_interval.tick() => {
                        if blackout_clone.load(Ordering::Relaxed) {
                            stream = None;
                            update_conn_state(&state_clone, &reachable_clone, &blackout_clone, false);
                            continue;
                        }
                        if let Some(s) = stream.as_ref() {
                            let mut buf = [0u8; 1];
                            match s.try_read(&mut buf) {
                                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                    update_conn_state(&state_clone, &reachable_clone, &blackout_clone, true);
                                }
                                _ => {
                                    stream = None;
                                    update_conn_state(&state_clone, &reachable_clone, &blackout_clone, false);
                                }
                            }
                        } else if let Some(s) = try_connect(&addr_clone).await {
                            stream = Some(s);
                            update_conn_state(&state_clone, &reachable_clone, &blackout_clone, true);
                        } else {
                            update_conn_state(&state_clone, &reachable_clone, &blackout_clone, false);
                        }
                    }
                }
            }
        });

        Self {
            target_hub_addr,
            state,
            manual_blackout,
            hub_reachable,
            frames_sent,
            cmd_tx,
            ack_rx: Mutex::new(ack_rx),
            _runtime: runtime,
        }
    }

    pub fn target_hub_addr(&self) -> &str {
        &self.target_hub_addr
    }

    pub fn connection_state(&self) -> ConnectionState {
        if self.manual_blackout.load(Ordering::Relaxed) {
            return ConnectionState::Offline;
        }
        self.state
            .lock()
            .map(|g| *g)
            .unwrap_or(ConnectionState::Offline)
    }

    pub fn is_manual_blackout(&self) -> bool {
        self.manual_blackout.load(Ordering::Relaxed)
    }

    pub fn is_hub_reachable(&self) -> bool {
        self.hub_reachable.load(Ordering::Relaxed)
    }

    pub fn toggle_connection_state(&self) -> ConnectionState {
        let now_blackout = !self.manual_blackout.load(Ordering::Relaxed);
        self.manual_blackout.store(now_blackout, Ordering::Relaxed);
        let _ = self.cmd_tx.send(NetCommand::Probe);
        if now_blackout {
            if let Ok(mut guard) = self.state.lock() {
                *guard = ConnectionState::Offline;
            }
            ConnectionState::Offline
        } else if self.hub_reachable.load(Ordering::Relaxed) {
            if let Ok(mut guard) = self.state.lock() {
                *guard = ConnectionState::Online;
            }
            ConnectionState::Online
        } else {
            ConnectionState::Offline
        }
    }

    pub fn trigger_probe(&self) {
        let _ = self.cmd_tx.send(NetCommand::Probe);
    }

    pub fn frames_sent(&self) -> usize {
        self.frames_sent.load(Ordering::Relaxed)
    }

    pub fn queue_frame(&self, frame: OutboundFrame) -> bool {
        self.cmd_tx.send(NetCommand::Send(frame)).is_ok()
    }

    pub fn try_recv_ack(&self) -> Option<AckEvent> {
        self.ack_rx.lock().ok()?.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::net::TcpListener;

    #[test]
    fn test_wire_format_real_delivery_and_blackout() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();

        let (received_tx, received_rx) = std::sync::mpsc::channel::<String>();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.spawn(async move {
            let tokio_listener = TcpListener::from_std(listener).unwrap();
            if let Ok((sock, _)) = tokio_listener.accept().await {
                let mut lines = BufReader::new(sock).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = received_tx.send(line);
                }
            }
        });

        let client = BiomonLanClient::new(addr.to_string());
        assert!(client.queue_frame(OutboundFrame {
            crew_id: "AST-01".to_string(),
            packet_type: "TELEMETRY_SYNC".to_string(),
            payload: serde_json::json!({ "seq_id": 42, "hr_bpm": 88.0 }),
            sync_batch: Some(SyncBatch {
                telemetry_seq_ids: vec![42],
                ..Default::default()
            }),
        }));

        let line = received_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("should receive JSONL packet over TCP");
        let parsed: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(parsed["crew_id"], "AST-01");
        assert_eq!(parsed["packet_type"], "TELEMETRY_SYNC");
        assert_eq!(parsed["payload"]["seq_id"], 42);
        assert!(parsed.get("sync_batch").is_none());

        // Wait briefly for AckEvent
        let mut delivered_ack = None;
        for _ in 0..40 {
            if let Some(ack) = client.try_recv_ack() {
                delivered_ack = Some(ack);
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        let ack = delivered_ack.expect("expected ACK event after TCP write");
        assert!(ack.delivered);
        assert_eq!(client.frames_sent(), 1);
        assert_eq!(client.connection_state(), ConnectionState::Online);

        // Toggle blackout -> goes Offline and does NOT transmit or fake-increment frames_sent
        assert_eq!(client.toggle_connection_state(), ConnectionState::Offline);
        assert!(client.queue_frame(OutboundFrame {
            crew_id: "AST-01".to_string(),
            packet_type: "TELEMETRY_SYNC".to_string(),
            payload: serde_json::json!({ "seq_id": 43 }),
            sync_batch: None,
        }));

        let mut blackout_ack = None;
        for _ in 0..40 {
            if let Some(ack) = client.try_recv_ack() {
                blackout_ack = Some(ack);
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        let ack2 = blackout_ack.expect("expected negative ACK during blackout");
        assert!(!ack2.delivered);
        assert_eq!(client.frames_sent(), 1);
        assert!(received_rx.try_recv().is_err());
    }
}
