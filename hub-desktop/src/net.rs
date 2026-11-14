use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundPacket {
    pub crew_id: String,
    pub packet_type: String,
    pub payload: serde_json::Value,
}

impl InboundPacket {
    pub fn from_json_line(line: &str) -> serde_json::Result<Self> {
        serde_json::from_str(line)
    }
}

pub struct HubLanServer {
    bind_addr: String,
    is_ready: Arc<AtomicBool>,
    connected_clients: Arc<AtomicUsize>,
    inbound_tx: mpsc::UnboundedSender<InboundPacket>,
    inbound_rx: mpsc::UnboundedReceiver<InboundPacket>,
    _runtime: tokio::runtime::Runtime,
}

impl HubLanServer {
    pub fn start(bind_addr: impl Into<String>) -> Self {
        let bind_addr = bind_addr.into();
        let is_ready = Arc::new(AtomicBool::new(false));
        let connected_clients = Arc::new(AtomicUsize::new(0));
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel::<InboundPacket>();

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to build HubLanServer tokio runtime");

        let addr_clone = bind_addr.clone();
        let ready_clone = Arc::clone(&is_ready);
        let clients_clone = Arc::clone(&connected_clients);
        let tx_clone = inbound_tx.clone();

        runtime.spawn(async move {
            if let Ok(listener) = TcpListener::bind(&addr_clone).await {
                ready_clone.store(true, Ordering::SeqCst);
                while let Ok((socket, _peer)) = listener.accept().await {
                    clients_clone.fetch_add(1, Ordering::SeqCst);
                    let client_counter = Arc::clone(&clients_clone);
                    let stream_tx = tx_clone.clone();
                    tokio::spawn(async move {
                        let reader = BufReader::new(socket);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            let trimmed = line.trim();
                            if !trimmed.is_empty()
                                && let Ok(pkt) = InboundPacket::from_json_line(trimmed)
                            {
                                let _ = stream_tx.send(pkt);
                            }
                        }
                        client_counter.fetch_sub(1, Ordering::SeqCst);
                    });
                }
            }
        });

        Self {
            bind_addr,
            is_ready,
            connected_clients,
            inbound_tx,
            inbound_rx,
            _runtime: runtime,
        }
    }

    pub fn bind_addr(&self) -> &str {
        &self.bind_addr
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready.load(Ordering::SeqCst)
    }

    pub fn connected_clients(&self) -> usize {
        self.connected_clients.load(Ordering::SeqCst)
    }

    pub fn inject_test_packet(&self, pkt: InboundPacket) {
        let _ = self.inbound_tx.send(pkt);
    }

    pub fn try_recv_packet(&mut self) -> Option<InboundPacket> {
        self.inbound_rx.try_recv().ok()
    }
}
