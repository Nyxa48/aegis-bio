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
use tokio::sync::{mpsc, watch};

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
    shutdown_tx: watch::Sender<bool>,
}

struct SlotClaim<'a> {
    session_id: u64,
    peer: SocketAddr,
    requested_crew_id: &'a str,
    requested_callsign: &'a str,
    device_id: Option<String>,
    tx: mpsc::UnboundedSender<HubDownlinkMessage>,
    shutdown_tx: watch::Sender<bool>,
}

fn claim_open_slot(
    sessions: &Arc<Mutex<HashMap<SocketAddr, ClientSession>>>,
    claim: SlotClaim<'_>,
) -> Option<(String, String)> {
    let mut map = sessions.lock().ok()?;

    // If the exact same device reconnected before the previous socket timed out,
    // replace and signal its stale socket entry to close immediately so it can
    // reclaim its slot cleanly without leaving a ghost reader running.
    if let Some(ref dev) = claim.device_id {
        let stale_peers: Vec<SocketAddr> = map
            .iter()
            .filter(|(_, s)| {
                s.session_id != claim.session_id && s.device_id.as_deref() == Some(dev.as_str())
            })
            .map(|(p, _)| *p)
            .collect();
        for sp in stale_peers {
            if let Some(old) = map.remove(&sp) {
                let _ = old.shutdown_tx.send(true);
            }
        }
    }

    let start_idx = CREW_SLOTS
        .iter()
        .position(|(id, _)| *id == claim.requested_crew_id)
        .unwrap_or(0);

    for offset in 0..MAX_ASTRONAUT_SLOTS {
        let idx = (start_idx + offset) % MAX_ASTRONAUT_SLOTS;
        let (cand_id, default_callsign) = CREW_SLOTS[idx];
        let occupied = map
            .values()
            .any(|s| s.session_id != claim.session_id && s.crew_id == cand_id);
        if !occupied {
            let assigned_crew = cand_id.to_string();
            let assigned_callsign =
                if cand_id == claim.requested_crew_id && !claim.requested_callsign.is_empty() {
                    claim.requested_callsign.to_string()
                } else {
                    default_callsign.to_string()
                };
            map.insert(
                claim.peer,
                ClientSession {
                    session_id: claim.session_id,
                    crew_id: assigned_crew.clone(),
                    callsign: assigned_callsign.clone(),
                    device_id: claim.device_id,
                    tx: claim.tx,
                    shutdown_tx: claim.shutdown_tx,
                },
            );
            return Some((assigned_crew, assigned_callsign));
        }
    }

    None
}

fn is_rfc1918_v4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    (o[0] == 192 && o[1] == 168) || (o[0] == 172 && (16..=31).contains(&o[1])) || o[0] == 10
}

fn is_cgnat_v4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 100 && (64..=127).contains(&o[1])
}

#[cfg(target_os = "windows")]
fn collect_windows_adapter_ipv4s(out: &mut Vec<Ipv4Addr>) {
    use windows::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
    use windows::Win32::NetworkManagement::IpHelper::{
        GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST,
        GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};

    let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
    let mut size: u32 = 16 * 1024;
    let mut buf = vec![0u8; size as usize];

    for _ in 0..3 {
        let rc = unsafe {
            GetAdaptersAddresses(
                AF_INET.0 as u32,
                flags,
                None,
                Some(buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
                &mut size,
            )
        };
        if rc == 0 {
            let mut cur = buf.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
            while !cur.is_null() {
                let adapter = unsafe { &*cur };
                if adapter.OperStatus == IfOperStatusUp {
                    let mut uc = adapter.FirstUnicastAddress;
                    while !uc.is_null() {
                        let ua = unsafe { &*uc };
                        if !ua.Address.lpSockaddr.is_null() {
                            let sa_fam = unsafe { (*ua.Address.lpSockaddr).sa_family };
                            if sa_fam == AF_INET {
                                let sa_in =
                                    unsafe { &*(ua.Address.lpSockaddr as *const SOCKADDR_IN) };
                                let ip = Ipv4Addr::from(u32::from_be(unsafe {
                                    sa_in.sin_addr.S_un.S_addr
                                }));
                                if !ip.is_unspecified()
                                    && !ip.is_loopback()
                                    && !ip.is_multicast()
                                    && !out.contains(&ip)
                                {
                                    out.push(ip);
                                }
                            }
                        }
                        uc = ua.Next;
                    }
                }
                cur = adapter.Next;
            }
            return;
        } else if rc == ERROR_BUFFER_OVERFLOW.0 {
            buf.resize(size as usize, 0);
        } else {
            return;
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn collect_windows_adapter_ipv4s(_out: &mut Vec<Ipv4Addr>) {}

/// Enumerates active local IPv4 addresses via Windows adapter tables + OS route probes,
/// prioritizing Windows Mobile Hotspot (`192.168.137.1`), real RFC-1918 LAN IPs, and
/// placing host-only virtual / CGNAT tunnel interfaces last.
pub fn detect_local_ipv4s() -> Vec<String> {
    let mut ips: Vec<Ipv4Addr> = Vec::new();
    collect_windows_adapter_ipv4s(&mut ips);

    for probe in [
        "192.168.137.1:80",
        "192.168.1.1:80",
        "192.168.0.1:80",
        "192.168.43.1:80",
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
            && !ips.contains(&v4)
        {
            ips.push(v4);
        }
    }

    ips.sort_by_key(|ip| {
        let o = ip.octets();
        if *ip == Ipv4Addr::new(192, 168, 137, 1) {
            0_u8
        } else if o[0] == 192 && o[1] == 168 && o[2] == 137 {
            1_u8
        } else if is_rfc1918_v4(*ip) && !(o[0] == 192 && o[1] == 168 && o[2] == 56) {
            2_u8
        } else if is_rfc1918_v4(*ip) {
            3_u8
        } else if !ip.is_link_local() && !is_cgnat_v4(*ip) {
            4_u8
        } else if ip.is_link_local() {
            5_u8
        } else {
            6_u8
        }
    });

    // Drop link-local unless nothing else exists or a 169.254 interface is the only AP fallback
    let has_routable = ips.iter().any(|ip| !ip.is_link_local());
    let mut found: Vec<String> = ips
        .into_iter()
        .filter(|ip| !has_routable || !ip.is_link_local())
        .map(|ip| ip.to_string())
        .collect();

    if found.is_empty() {
        found.push(Ipv4Addr::LOCALHOST.to_string());
    }
    found
}

pub struct HubLanServer {
    bind_addr: String,
    tcp_port: Arc<AtomicU16>,
    discovery_port: Arc<AtomicU16>,
    local_ipv4s: Arc<RwLock<Vec<String>>>,
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
        let local_ipv4s = Arc::new(RwLock::new(detect_local_ipv4s()));
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

                    let client_counter = connected_clients_ref(&clients_clone);
                    let sessions_inner = Arc::clone(&sessions_clone);
                    let meta_inner = Arc::clone(&meta_for_tcp);
                    let event_tx_inner = tx_clone.clone();
                    let my_session_id = next_session_id.fetch_add(1, Ordering::Relaxed);

                    tokio::spawn(async move {
                        let (read_half, mut write_half) = tokio::io::split(socket);
                        let (out_tx, mut out_rx) = mpsc::unbounded_channel::<HubDownlinkMessage>();
                        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

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

                        let evicted = loop {
                            if *shutdown_rx.borrow() {
                                break true;
                            }
                            let next = tokio::select! {
                                biased;
                                res = shutdown_rx.changed() => {
                                    if res.is_ok() && *shutdown_rx.borrow() {
                                        break true;
                                    }
                                    continue;
                                }
                                res = tokio::time::timeout(Duration::from_secs(12), reader.next_line()) => res,
                            };
                            let Ok(Ok(Some(line))) = next else {
                                break false;
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
                                        SlotClaim {
                                            session_id: my_session_id,
                                            peer,
                                            requested_crew_id: &pkt.crew_id,
                                            requested_callsign: &req_callsign,
                                            device_id: dev_id,
                                            tx: out_tx.clone(),
                                            shutdown_tx: shutdown_tx.clone(),
                                        },
                                    ) {
                                        Some((assigned_crew, assigned_callsign)) => {
                                            if registered_crew.as_deref()
                                                != Some(assigned_crew.as_str())
                                            {
                                                registered_crew = Some(assigned_crew.clone());
                                                let _ = event_tx_inner.send(
                                                    HubNetEvent::ClientConnected {
                                                        peer,
                                                        crew_id: assigned_crew.clone(),
                                                        callsign: assigned_callsign.clone(),
                                                    },
                                                );
                                            }
                                            pkt.crew_id = assigned_crew.clone();
                                            if pkt.packet_type == "HELLO" {
                                                let m = meta_inner
                                                    .read()
                                                    .map(|g| g.clone())
                                                    .unwrap_or_default();
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
                                            break false;
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
                                    let m = meta_inner
                                        .read()
                                        .map(|g| g.clone())
                                        .unwrap_or_default();
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
                        };

                        drop(out_tx);
                        tokio::time::sleep(Duration::from_millis(15)).await;
                        writer_handle.abort();

                        if let Ok(mut map) = sessions_inner.lock()
                            && let Some(existing) = map.get(&peer)
                            && existing.session_id == my_session_id
                        {
                            map.remove(&peer);
                        }
                        if !evicted && let Some(crew_id) = registered_crew {
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

    pub fn local_ipv4s(&self) -> Vec<String> {
        self.local_ipv4s
            .read()
            .map(|g| g.clone())
            .unwrap_or_else(|_| vec![Ipv4Addr::LOCALHOST.to_string()])
    }

    pub fn refresh_local_ipv4s(&self) -> Vec<String> {
        let fresh = detect_local_ipv4s();
        if let Ok(mut g) = self.local_ipv4s.write() {
            *g = fresh.clone();
        }
        fresh
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
