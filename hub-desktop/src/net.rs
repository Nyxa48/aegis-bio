use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{
    IpAddr, Ipv4Addr, SocketAddr, TcpListener as StdTcpListener, UdpSocket as StdUdpSocket,
};
use std::sync::{
    Arc, Mutex, RwLock,
    atomic::{AtomicBool, AtomicU16, AtomicU64, AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::mpsc;

pub const DEFAULT_TCP_PORT: u16 = 7878;
pub const DEFAULT_DISCOVERY_PORT: u16 = 7879;
pub const DISCOVERY_MAGIC_REQ: &str = "AEGIS_HUB_DISCOVER";
pub const DISCOVERY_MAGIC_RESP: &str = "AEGIS_HUB_BEACON";
pub const MAX_ASTRONAUT_SLOTS: usize = 6;
pub const CREW_SLOTS: [(&str, &str); MAX_ASTRONAUT_SLOTS] = [
    ("AST-01", "ARES-1"),
    ("AST-02", "ARES-2"),
    ("AST-03", "ARES-3"),
    ("AST-04", "ARES-4"),
    ("AST-05", "ARES-5"),
    ("AST-06", "ARES-6"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryProbe {
    pub kind: String,
    pub proto: u8,
    pub crew_id: String,
    #[serde(default)]
    pub callsign: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryBeacon {
    pub kind: String,
    pub proto: u8,
    pub hub_id: String,
    pub tcp_port: u16,
    pub mission_phase: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HubDownlinkMessage {
    HelloAck {
        hub_id: String,
        mission_phase: String,
        distance_au: f64,
        one_way_light_min: f64,
        #[serde(default)]
        assigned_crew_id: String,
        #[serde(default)]
        assigned_callsign: String,
    },
    SlotFull {
        max_slots: usize,
        reason: String,
    },
    SyncAck {
        batch_id: u64,
        synced_records: usize,
    },
    PhaseSync {
        phase: String,
        apply_to_all: bool,
        target_crew_id: Option<String>,
    },
    EphemerisSync {
        mission_date: String,
        distance_au: f64,
        one_way_light_min: f64,
    },
}

impl HubDownlinkMessage {
    pub fn to_json_line(&self) -> serde_json::Result<String> {
        let mut line = serde_json::to_string(self)?;
        line.push('\n');
        Ok(line)
    }
}

#[derive(Debug, Clone)]
pub enum HubNetEvent {
    ClientConnected {
        peer: SocketAddr,
        crew_id: String,
        callsign: String,
    },
    Packet {
        peer: SocketAddr,
        packet: InboundPacket,
    },
    ClientDisconnected {
        peer: SocketAddr,
        crew_id: String,
    },
}

#[derive(Debug, Clone)]
pub struct HubAdvertisedMeta {
    pub hub_id: String,
    pub mission_phase: String,
    pub distance_au: f64,
    pub one_way_light_min: f64,
}

impl Default for HubAdvertisedMeta {
    fn default() -> Self {
        Self {
            hub_id: "AEGIS-HUB-01".to_string(),
            mission_phase: "ORBIT".to_string(),
            distance_au: 1.324362,
            one_way_light_min: 11.0144,
        }
    }
}

#[derive(Debug, Clone)]
struct ClientSession {
    session_id: u64,
    crew_id: String,
    callsign: String,
    device_id: Option<String>,
    tx: mpsc::UnboundedSender<HubDownlinkMessage>,
}

fn claim_open_slot(
    sessions: &Arc<Mutex<HashMap<SocketAddr, ClientSession>>>,
    session_id: u64,
    peer: SocketAddr,
    requested_crew_id: &str,
    requested_callsign: &str,
    device_id: Option<String>,
    tx: mpsc::UnboundedSender<HubDownlinkMessage>,
) -> Option<(String, String)> {
    let mut map = sessions.lock().ok()?;

    // If the exact same device reconnected before the previous socket timed out,
    // replace its stale socket entry so it can reclaim its slot cleanly.
    if let Some(ref dev) = device_id {
        let stale_peers: Vec<SocketAddr> = map
            .iter()
            .filter(|(_, s)| s.session_id != session_id && s.device_id.as_deref() == Some(dev.as_str()))
            .map(|(p, _)| *p)
            .collect();
        for sp in stale_peers {
            map.remove(&sp);
        }
    }

    let start_idx = CREW_SLOTS
        .iter()
        .position(|(id, _)| *id == requested_crew_id)
        .unwrap_or(0);

    for offset in 0..MAX_ASTRONAUT_SLOTS {
        let idx = (start_idx + offset) % MAX_ASTRONAUT_SLOTS;
        let (cand_id, default_callsign) = CREW_SLOTS[idx];
        let occupied = map
            .values()
            .any(|s| s.session_id != session_id && s.crew_id == cand_id);
        if !occupied {
            let assigned_crew = cand_id.to_string();
            let assigned_callsign = if cand_id == requested_crew_id && !requested_callsign.is_empty() {
                requested_callsign.to_string()
            } else {
                default_callsign.to_string()
            };
            map.insert(
                peer,
                ClientSession {
                    session_id,
                    crew_id: assigned_crew.clone(),
                    callsign: assigned_callsign.clone(),
                    device_id,
                    tx,
                },
            );
            return Some((assigned_crew, assigned_callsign));
        }
    }

    None
}

/// Pure-Rust OS routing-table probe to discover active non-loopback local IPv4 addresses.
/// UDP `connect` only inspects kernel route selection and transmits zero packets.
pub fn detect_local_ipv4s() -> Vec<String> {
    let mut found = Vec::new();
    for probe in [
        "192.168.1.1:80",
        "192.168.0.1:80",
        "192.168.43.1:80",
        "192.168.137.1:80",
        "172.20.10.1:80",
        "10.0.0.1:80",
        "172.16.0.1:80",
        "192.0.2.1:80",
        "1.1.1.1:80",
    ] {
        if let Ok(sock) = StdUdpSocket::bind("0.0.0.0:0")
            && sock.connect(probe).is_ok()
            && let Ok(IpAddr::V4(v4)) = sock.local_addr().map(|a| a.ip())
            && !v4.is_unspecified()
            && !v4.is_loopback()
            && !v4.is_link_local()
        {
            let s = v4.to_string();
            if !found.contains(&s) {
                found.push(s);
            }
        }
    }
    if found.is_empty() {
        found.push(Ipv4Addr::LOCALHOST.to_string());
    }
    found
}

pub struct HubLanServer {
    bind_addr: String,
    tcp_port: Arc<AtomicU16>,
    discovery_port: Arc<AtomicU16>,
    local_ipv4s: Vec<String>,
    is_ready: Arc<AtomicBool>,
    discovery_ready: Arc<AtomicBool>,
    last_error: Arc<RwLock<Option<String>>>,
    connected_clients: Arc<AtomicUsize>,
    sessions: Arc<Mutex<HashMap<SocketAddr, ClientSession>>>,
    meta: Arc<RwLock<HubAdvertisedMeta>>,
    event_tx: mpsc::UnboundedSender<HubNetEvent>,
    event_rx: mpsc::UnboundedReceiver<HubNetEvent>,
    _runtime: tokio::runtime::Runtime,
}

impl HubLanServer {
    pub fn start(bind_addr: impl Into<String>) -> Self {
        let bind_str = bind_addr.into();
        let (bind_ip, tcp_port) = match bind_str.parse::<SocketAddr>() {
            Ok(sa) => (sa.ip().to_string(), sa.port()),
            Err(_) => ("0.0.0.0".to_string(), DEFAULT_TCP_PORT),
        };
        Self::start_with_ports(&bind_ip, tcp_port, DEFAULT_DISCOVERY_PORT)
    }

    pub fn start_with_ports(bind_ip: &str, req_tcp_port: u16, req_discovery_port: u16) -> Self {
        let local_ipv4s = detect_local_ipv4s();
        let tcp_port_atomic = Arc::new(AtomicU16::new(req_tcp_port));
        let disc_port_atomic = Arc::new(AtomicU16::new(req_discovery_port));
        let is_ready = Arc::new(AtomicBool::new(false));
        let discovery_ready = Arc::new(AtomicBool::new(false));
        let last_error = Arc::new(RwLock::new(None));
        let connected_clients = Arc::new(AtomicUsize::new(0));
        let sessions = Arc::new(Mutex::new(HashMap::<SocketAddr, ClientSession>::new()));
        let meta = Arc::new(RwLock::new(HubAdvertisedMeta::default()));
        let (event_tx, event_rx) = mpsc::unbounded_channel::<HubNetEvent>();

        let std_tcp = match StdTcpListener::bind(format!("{bind_ip}:{req_tcp_port}")) {
            Ok(l) => {
                if let Ok(local) = l.local_addr() {
                    tcp_port_atomic.store(local.port(), Ordering::SeqCst);
                }
                let _ = l.set_nonblocking(true);
                is_ready.store(true, Ordering::SeqCst);
                Some(l)
            }
            Err(e) => {
                if let Ok(mut err_g) = last_error.write() {
                    *err_g = Some(format!("TCP :{req_tcp_port} bind error: {e}"));
                }
                None
            }
        };

        let std_udp = match StdUdpSocket::bind(format!("{bind_ip}:{req_discovery_port}")) {
            Ok(u) => {
                if let Ok(local) = u.local_addr() {
                    disc_port_atomic.store(local.port(), Ordering::SeqCst);
                }
                let _ = u.set_broadcast(true);
                let _ = u.set_nonblocking(true);
                discovery_ready.store(true, Ordering::SeqCst);
                Some(u)
            }
            Err(e) => {
                if let Ok(mut err_g) = last_error.write()
                    && err_g.is_none()
                {
                    *err_g = Some(format!("UDP :{req_discovery_port} bind error: {e}"));
                }
                None
            }
        };

        let bind_addr = format!("{bind_ip}:{}", tcp_port_atomic.load(Ordering::SeqCst));

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to build HubLanServer tokio runtime");

        let tcp_port_for_udp = Arc::clone(&tcp_port_atomic);
        let meta_for_udp = Arc::clone(&meta);
        let err_for_udp = Arc::clone(&last_error);

        let clients_clone = Arc::clone(&connected_clients);
        let sessions_clone = Arc::clone(&sessions);
        let meta_for_tcp = Arc::clone(&meta);
        let tx_clone = event_tx.clone();
        let next_session_id = Arc::new(AtomicU64::new(1));

        runtime.spawn(async move {
            // 1. UDP Discovery Responder Task
            if let Some(udp_std) = std_udp
                && let Ok(udp) = UdpSocket::from_std(udp_std)
            {
                tokio::spawn(async move {
                    let mut buf = [0u8; 2048];
                    loop {
                        match udp.recv_from(&mut buf).await {
                            Ok((n, peer)) => {
                                if let Ok(text) = std::str::from_utf8(&buf[..n])
                                    && let Ok(probe) =
                                        serde_json::from_str::<DiscoveryProbe>(text.trim())
                                    && probe.kind == DISCOVERY_MAGIC_REQ
                                {
                                    let (hub_id, mission_phase) = meta_for_udp
                                        .read()
                                        .map(|m| (m.hub_id.clone(), m.mission_phase.clone()))
                                        .unwrap_or_else(|_| {
                                            ("AEGIS-HUB-01".to_string(), "ORBIT".to_string())
                                        });
                                    let beacon = DiscoveryBeacon {
                                        kind: DISCOVERY_MAGIC_RESP.to_string(),
                                        proto: 1,
                                        hub_id,
                                        tcp_port: tcp_port_for_udp.load(Ordering::SeqCst),
                                        mission_phase,
                                    };
                                    if let Ok(payload) = serde_json::to_vec(&beacon) {
                                        let _ = udp.send_to(&payload, peer).await;
                                    }
                                }
                            }
                            Err(e) => {
                                if let Ok(mut g) = err_for_udp.write() {
                                    *g = Some(format!("UDP discovery error: {e}"));
                                }
                                tokio::time::sleep(Duration::from_millis(50)).await;
                            }
                        }
                    }
                });
            }

            // 2. TCP Bidirectional Session Accept Loop
            if let Some(tcp_std) = std_tcp
                && let Ok(listener) = TcpListener::from_std(tcp_std)
            {
                loop {
                    let Ok((socket, peer)) = listener.accept().await else {
                        tokio::time::sleep(Duration::from_millis(20)).await;
                        continue;
                    };
                    let _ = socket.set_nodelay(true);
                    clients_clone.fetch_add(1, Ordering::SeqCst);

                    let client_counter = Arc::clone(&connected_clients_ref(&clients_clone));
                    let sessions_inner = Arc::clone(&sessions_clone);
                    let meta_inner = Arc::clone(&meta_for_tcp);
                    let event_tx_inner = tx_clone.clone();
                    let my_session_id = next_session_id.fetch_add(1, Ordering::Relaxed);

                    tokio::spawn(async move {
                        let (read_half, mut write_half) = tokio::io::split(socket);
                        let (out_tx, mut out_rx) = mpsc::unbounded_channel::<HubDownlinkMessage>();

                        let writer_handle = tokio::spawn(async move {
                            while let Some(msg) = out_rx.recv().await {
                                if let Ok(line) = msg.to_json_line()
                                    && (write_half.write_all(line.as_bytes()).await.is_err()
                                        || write_half.flush().await.is_err())
                                {
                                    break;
                                }
                            }
                        });

                        let mut registered_crew: Option<String> = None;
                        let mut reader = BufReader::new(read_half).lines();

                        loop {
                            let next =
                                tokio::time::timeout(Duration::from_secs(12), reader.next_line())
                                    .await;
                            let Ok(Ok(Some(line))) = next else {
                                break;
                            };

                            let trimmed = line.trim();
                            if trimmed.is_empty() {
                                continue;
                            }
                            let Ok(mut pkt) = InboundPacket::from_json_line(trimmed) else {
                                continue;
                            };

                            if pkt.packet_type == "HELLO" || registered_crew.is_none() {
                                let req_callsign = pkt
                                    .payload
                                    .get("callsign")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let dev_id = pkt
                                    .payload
                                    .get("device_id")
                                    .and_then(|v| v.as_str())
                                    .map(str::to_string);

                                let slot_changed = registered_crew.is_none()
                                    || (pkt.packet_type == "HELLO"
                                        && registered_crew.as_deref() != Some(pkt.crew_id.as_str()));

                                if slot_changed {
                                    match claim_open_slot(
                                        &sessions_inner,
                                        my_session_id,
                                        peer,
                                        &pkt.crew_id,
                                        &req_callsign,
                                        dev_id,
                                        out_tx.clone(),
                                    ) {
                                        Some((assigned_crew, assigned_callsign)) => {
                                            if registered_crew.as_deref() != Some(assigned_crew.as_str()) {
                                                registered_crew = Some(assigned_crew.clone());
                                                let _ = event_tx_inner.send(HubNetEvent::ClientConnected {
                                                    peer,
                                                    crew_id: assigned_crew.clone(),
                                                    callsign: assigned_callsign.clone(),
                                                });
                                            }
                                            pkt.crew_id = assigned_crew.clone();
                                            if pkt.packet_type == "HELLO" {
                                                let m = meta_inner.read().map(|g| g.clone()).unwrap_or_default();
                                                let _ = out_tx.send(HubDownlinkMessage::HelloAck {
                                                    hub_id: m.hub_id,
                                                    mission_phase: m.mission_phase,
                                                    distance_au: m.distance_au,
                                                    one_way_light_min: m.one_way_light_min,
                                                    assigned_crew_id: assigned_crew,
                                                    assigned_callsign,
                                                });
                                            }
                                        }
                                        None => {
                                            let _ = out_tx.send(HubDownlinkMessage::SlotFull {
                                                max_slots: MAX_ASTRONAUT_SLOTS,
                                                reason: "ALL 6 ASTRONAUT SLOTS OCCUPIED".to_string(),
                                            });
                                            tokio::time::sleep(Duration::from_millis(120)).await;
                                            break;
                                        }
                                    }
                                } else if pkt.packet_type == "HELLO"
                                    && let Some(ref cur_crew) = registered_crew
                                {
                                    let cur_callsign = sessions_inner
                                        .lock()
                                        .ok()
                                        .and_then(|map| map.get(&peer).map(|s| s.callsign.clone()))
                                        .unwrap_or_else(|| cur_crew.clone());
                                    pkt.crew_id = cur_crew.clone();
                                    let m = meta_inner.read().map(|g| g.clone()).unwrap_or_default();
                                    let _ = out_tx.send(HubDownlinkMessage::HelloAck {
                                        hub_id: m.hub_id,
                                        mission_phase: m.mission_phase,
                                        distance_au: m.distance_au,
                                        one_way_light_min: m.one_way_light_min,
                                        assigned_crew_id: cur_crew.clone(),
                                        assigned_callsign: cur_callsign,
                                    });
                                }
                            } else if let Some(ref assigned) = registered_crew {
                                // Pin subsequent packets on this socket to the Hub-assigned crew slot.
                                pkt.crew_id = assigned.clone();
                            }

                            let _ = event_tx_inner.send(HubNetEvent::Packet { peer, packet: pkt });
                        }

                        drop(out_tx);
                        tokio::time::sleep(Duration::from_millis(15)).await;
                        writer_handle.abort();

                        if let Ok(mut map) = sessions_inner.lock()
                            && let Some(existing) = map.get(&peer)
                            && existing.session_id == my_session_id
                        {
                            map.remove(&peer);
                        }
                        if let Some(crew_id) = registered_crew {
                            let _ = event_tx_inner
                                .send(HubNetEvent::ClientDisconnected { peer, crew_id });
                        }
                        client_counter.fetch_sub(1, Ordering::SeqCst);
                    });
                }
            }
        });

        Self {
            bind_addr,
            tcp_port: tcp_port_atomic,
            discovery_port: disc_port_atomic,
            local_ipv4s,
            is_ready,
            discovery_ready,
            last_error,
            connected_clients,
            sessions,
            meta,
            event_tx,
            event_rx,
            _runtime: runtime,
        }
    }

    pub fn bind_addr(&self) -> &str {
        &self.bind_addr
    }

    pub fn tcp_port(&self) -> u16 {
        self.tcp_port.load(Ordering::SeqCst)
    }

    pub fn discovery_port(&self) -> u16 {
        self.discovery_port.load(Ordering::SeqCst)
    }

    pub fn local_ipv4s(&self) -> &[String] {
        &self.local_ipv4s
    }

    pub fn discovery_ready(&self) -> bool {
        self.discovery_ready.load(Ordering::SeqCst)
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error.read().ok().and_then(|g| g.clone())
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready.load(Ordering::SeqCst)
    }

    pub fn connected_clients(&self) -> usize {
        self.connected_clients.load(Ordering::SeqCst)
    }

    pub fn online_crew_peers(&self) -> HashMap<String, String> {
        let mut out = HashMap::new();
        if let Ok(map) = self.sessions.lock() {
            for (peer, sess) in map.iter() {
                out.insert(
                    sess.crew_id.clone(),
                    format!("{} ({})", peer, sess.callsign),
                );
            }
        }
        out
    }

    pub fn crew_has_active_session(&self, crew_id: &str) -> bool {
        self.sessions
            .lock()
            .map(|map| map.values().any(|s| s.crew_id == crew_id))
            .unwrap_or(false)
    }

    pub fn update_hub_metadata(
        &self,
        mission_phase: &str,
        distance_au: f64,
        one_way_light_min: f64,
    ) {
        if let Ok(mut m) = self.meta.write() {
            m.mission_phase = mission_phase.to_string();
            m.distance_au = distance_au;
            m.one_way_light_min = one_way_light_min;
        }
    }

    pub fn send_to_crew(&self, crew_id: &str, msg: HubDownlinkMessage) -> usize {
        let mut sent = 0;
        if let Ok(map) = self.sessions.lock() {
            for sess in map.values() {
                if sess.crew_id == crew_id && sess.tx.send(msg.clone()).is_ok() {
                    sent += 1;
                }
            }
        }
        sent
    }

    pub fn broadcast_downlink(&self, msg: HubDownlinkMessage) -> usize {
        let mut sent = 0;
        if let Ok(map) = self.sessions.lock() {
            for sess in map.values() {
                if sess.tx.send(msg.clone()).is_ok() {
                    sent += 1;
                }
            }
        }
        sent
    }

    pub fn inject_test_packet(&self, pkt: InboundPacket) {
        let _ = self.event_tx.send(HubNetEvent::Packet {
            peer: SocketAddr::from(([127, 0, 0, 1], 0)),
            packet: pkt,
        });
    }

    pub fn try_recv_event(&mut self) -> Option<HubNetEvent> {
        self.event_rx.try_recv().ok()
    }

    pub fn try_recv_packet(&mut self) -> Option<InboundPacket> {
        while let Some(ev) = self.try_recv_event() {
            if let HubNetEvent::Packet { packet, .. } = ev {
                return Some(packet);
            }
        }
        None
    }
}

fn connected_clients_ref(src: &Arc<AtomicUsize>) -> Arc<AtomicUsize> {
    Arc::clone(src)
}
