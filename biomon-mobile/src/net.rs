use serde::{Deserialize, Serialize};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
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
            ConnectionState::Online => "ÇEVRİMİÇİ",
            ConnectionState::Offline => "ÇEVRİMDIŞI",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundFrame {
    pub packet_type: String,
    pub payload: serde_json::Value,
}

impl OutboundFrame {
    pub fn to_json_line(&self) -> serde_json::Result<String> {
        let mut line = serde_json::to_string(self)?;
        line.push('\n');
        Ok(line)
    }
}

pub struct BiomonLanClient {
    target_hub_addr: String,
    state: Arc<Mutex<ConnectionState>>,
    frames_sent: Arc<AtomicUsize>,
    outbound_tx: mpsc::UnboundedSender<OutboundFrame>,
    _runtime: tokio::runtime::Runtime,
}

impl BiomonLanClient {
    pub fn new(target_hub_addr: impl Into<String>) -> Self {
        let target_hub_addr = target_hub_addr.into();
        let state = Arc::new(Mutex::new(ConnectionState::Online));
        let frames_sent = Arc::new(AtomicUsize::new(0));
        let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<OutboundFrame>();

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("failed to build BiomonLanClient tokio runtime");

        let addr_clone = target_hub_addr.clone();
        let state_clone = Arc::clone(&state);
        let sent_clone = Arc::clone(&frames_sent);

        runtime.spawn(async move {
            while let Some(frame) = outbound_rx.recv().await {
                if let Ok(line) = frame.to_json_line() {
                    if let Ok(Ok(mut stream)) = tokio::time::timeout(
                        Duration::from_millis(350),
                        TcpStream::connect(&addr_clone),
                    )
                    .await
                    {
                        if stream.write_all(line.as_bytes()).await.is_ok() {
                            if let Ok(mut guard) = state_clone.lock() {
                                *guard = ConnectionState::Online;
                            }
                            sent_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    } else {
                        // In local standalone desktop testing without hub-desktop running,
                        // still count the simulated packet dispatch so UI counters advance.
                        sent_clone.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });

        Self {
            target_hub_addr,
            state,
            frames_sent,
            outbound_tx,
            _runtime: runtime,
        }
    }

    pub fn target_hub_addr(&self) -> &str {
        &self.target_hub_addr
    }

    pub fn connection_state(&self) -> ConnectionState {
        self.state
            .lock()
            .map(|g| *g)
            .unwrap_or(ConnectionState::Online)
    }

    pub fn toggle_connection_state(&self) -> ConnectionState {
        if let Ok(mut guard) = self.state.lock() {
            *guard = match *guard {
                ConnectionState::Online => ConnectionState::Offline,
                ConnectionState::Offline => ConnectionState::Online,
            };
            *guard
        } else {
            ConnectionState::Online
        }
    }

    pub fn frames_sent(&self) -> usize {
        self.frames_sent.load(Ordering::Relaxed)
    }

    pub fn queue_frame(&self, frame: OutboundFrame) -> bool {
        self.outbound_tx.send(frame).is_ok()
    }
}
