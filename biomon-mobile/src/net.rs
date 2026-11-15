use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket as StdUdpSocket};
use std::sync::{
    Arc, Mutex, RwLock,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static NEXT_CLIENT_SEQ: AtomicU64 = AtomicU64::new(1);

fn next_device_id() -> String {
    let seq = NEXT_CLIENT_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("dev-{}-{}-{}", std::process::id(), seq, nanos)
}
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::{Notify, mpsc};

pub const DEFAULT_TCP_PORT: u16 = 7878;
pub const DEFAULT_DISCOVERY_PORT: u16 = 7879;
pub const DISCOVERY_MAGIC_REQ: &str = "AEGIS_HUB_DISCOVER";
pub const DISCOVERY_MAGIC_RESP: &str = "AEGIS_HUB_BEACON";

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
pub struct DiscoveryProbe {
    pub kind: String,
    pub proto: u8,
    pub crew_id: String,
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
pub struct OutboundFrame {
    pub crew_id: String,
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

#[derive(Debug, Clone)]
pub enum LinkEvent {
    Connected(String),
    Disconnected,
    Downlink(HubDownlinkMessage),
}

#[derive(Debug, Clone)]
pub struct ClientIdentity {
    pub crew_id: String,
    pub callsign: String,
    pub active_phase: String,
}

#[derive(Debug, Clone)]
pub struct BiomonLanConfig {
    pub hub_override_addr: Option<String>,
    pub discovery_port: u16,
    pub tcp_fallback_port: u16,
    pub crew_id: String,
    pub callsign: String,
    pub active_phase: String,
}

impl Default for BiomonLanConfig {
    fn default() -> Self {
        Self {
            hub_override_addr: std::env::var("AEGIS_HUB_ADDR")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("auto")),
            discovery_port: std::env::var("AEGIS_DISCOVERY_PORT")
                .ok()
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(DEFAULT_DISCOVERY_PORT),
            tcp_fallback_port: std::env::var("AEGIS_HUB_PORT")
                .ok()
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(DEFAULT_TCP_PORT),
            crew_id: "AST-01".to_string(),
            callsign: "ARES-1".to_string(),
            active_phase: "ORBIT".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LocalIpv4Iface {
    ip: Ipv4Addr,
    broadcast: Ipv4Addr,
}

fn is_rfc1918(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    (o[0] == 192 && o[1] == 168)
        || (o[0] == 172 && (16..=31).contains(&o[1]))
        || o[0] == 10
}

fn is_cgnat(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 100 && (64..=127).contains(&o[1])
}

fn push_iface(out: &mut Vec<LocalIpv4Iface>, ip: Ipv4Addr, broadcast: Option<Ipv4Addr>) {
    if ip.is_unspecified() || ip.is_loopback() || ip.is_link_local() || ip.is_multicast() {
        return;
    }
    let bcast = broadcast.unwrap_or_else(|| {
        let o = ip.octets();
        Ipv4Addr::new(o[0], o[1], o[2], 255)
    });
    if !out.iter().any(|x| x.ip == ip) {
        out.push(LocalIpv4Iface { ip, broadcast: bcast });
    }
}

#[cfg(unix)]
fn collect_unix_ifaces(out: &mut Vec<LocalIpv4Iface>) {
    unsafe {
        let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();
        if libc::getifaddrs(&mut ifap) != 0 || ifap.is_null() {
            return;
        }
        let mut cur = ifap;
        while !cur.is_null() {
            let ifa = &*cur;
            let flags = ifa.ifa_flags as i32;
            let is_up = (flags & libc::IFF_UP) != 0;
            let is_loop = (flags & libc::IFF_LOOPBACK) != 0;
            if is_up && !is_loop && !ifa.ifa_addr.is_null() && (*ifa.ifa_addr).sa_family as i32 == libc::AF_INET {
                let sa = &*(ifa.ifa_addr as *const libc::sockaddr_in);
                let ip = Ipv4Addr::from(u32::from_be(sa.sin_addr.s_addr));
                let bcast = if !ifa.ifa_netmask.is_null()
                    && (*ifa.ifa_netmask).sa_family as i32 == libc::AF_INET
                {
                    let nm = &*(ifa.ifa_netmask as *const libc::sockaddr_in);
                    let mask_u32 = u32::from_be(nm.sin_addr.s_addr);
                    let ip_u32 = u32::from_be(sa.sin_addr.s_addr);
                    if mask_u32 != 0 {
                        Some(Ipv4Addr::from((ip_u32 & mask_u32) | !mask_u32))
                    } else {
                        None
                    }
                } else {
                    None
                };
                push_iface(out, ip, bcast);
            }
            cur = ifa.ifa_next;
        }
        libc::freeifaddrs(ifap);
    }
}

#[cfg(not(unix))]
fn collect_unix_ifaces(_out: &mut Vec<LocalIpv4Iface>) {}

/// Enumerates active local IPv4 interfaces (Android/Linux `getifaddrs` + multi-subnet OS route probes),
/// sorted with private RFC-1918 LAN interfaces first so cellular interfaces never shadow Wi-Fi.
fn detect_local_ipv4_ifaces() -> Vec<LocalIpv4Iface> {
    let mut found = Vec::new();
    collect_unix_ifaces(&mut found);

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
        {
            push_iface(&mut found, v4, None);
        }
    }

    found.sort_by_key(|iface| {
        if is_rfc1918(iface.ip) {
            0_u8
        } else if !is_cgnat(iface.ip) {
            1_u8
        } else {
            2_u8
        }
    });
    found
}

pub fn detect_local_ipv4s() -> Vec<Ipv4Addr> {
    detect_local_ipv4_ifaces().into_iter().map(|i| i.ip).collect()
}

fn hello_frame(identity: &ClientIdentity, device_id: &str) -> OutboundFrame {
    OutboundFrame {
        crew_id: identity.crew_id.clone(),
        packet_type: "HELLO".to_string(),
        payload: json!({
            "callsign": identity.callsign,
            "active_phase": identity.active_phase,
            "device_id": device_id,
            "proto": 1,
        }),
    }
}

enum HandshakeOutcome {
    Accepted(TcpStream, HubDownlinkMessage),
    Rejected(HubDownlinkMessage),
    Failed,
}

fn heartbeat_frame(identity: &ClientIdentity) -> OutboundFrame {
    OutboundFrame {
        crew_id: identity.crew_id.clone(),
        packet_type: "HEARTBEAT".to_string(),
        payload: json!({
            "callsign": identity.callsign,
            "active_phase": identity.active_phase,
        }),
    }
}

async fn udp_probe_socket(
    bind_ip: Ipv4Addr,
    targets: Vec<SocketAddr>,
    probe_bytes: Vec<u8>,
    window: Duration,
) -> Option<SocketAddr> {
    let udp = UdpSocket::bind(SocketAddr::from((bind_ip, 0))).await.ok()?;
    let _ = udp.set_broadcast(true);
    for target in targets {
        let _ = udp.send_to(&probe_bytes, target).await;
    }

    let deadline = tokio::time::Instant::now() + window;
    let mut buf = [0u8; 2048];
    loop {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            return None;
        }
        let rem = deadline - now;
        match tokio::time::timeout(rem, udp.recv_from(&mut buf)).await {
            Ok(Ok((n, src))) => {
                if let Ok(text) = std::str::from_utf8(&buf[..n])
                    && let Ok(beacon) = serde_json::from_str::<DiscoveryBeacon>(text.trim())
                    && beacon.kind == DISCOVERY_MAGIC_RESP
                {
                    return Some(SocketAddr::new(src.ip(), beacon.tcp_port));
                }
            }
            Ok(Err(_)) => {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(_) => return None,
        }
    }
}

async fn discover_via_udp(discovery_port: u16, identity: &ClientIdentity) -> Option<SocketAddr> {
    let probe = DiscoveryProbe {
        kind: DISCOVERY_MAGIC_REQ.to_string(),
        proto: 1,
        crew_id: identity.crew_id.clone(),
        callsign: identity.callsign.clone(),
    };
    let probe_bytes = serde_json::to_vec(&probe).ok()?;
    let ifaces = detect_local_ipv4_ifaces();
    let window = Duration::from_millis(650);

    let mut default_targets: Vec<SocketAddr> = vec![
        SocketAddr::from(([255, 255, 255, 255], discovery_port)),
        SocketAddr::from(([127, 0, 0, 1], discovery_port)),
    ];
    for iface in &ifaces {
        let bcast = SocketAddr::from((iface.broadcast, discovery_port));
        if !default_targets.contains(&bcast) {
            default_targets.push(bcast);
        }
    }

    let (tx, mut rx) = mpsc::channel::<SocketAddr>(4);

    // 1. Default 0.0.0.0 socket probing limited + directed broadcasts + loopback
    let tx0 = tx.clone();
    let bytes0 = probe_bytes.clone();
    tokio::spawn(async move {
        if let Some(addr) = udp_probe_socket(Ipv4Addr::UNSPECIFIED, default_targets, bytes0, window).await {
            let _ = tx0.send(addr).await;
        }
    });

    // 2. One interface-pinned socket per real NIC (forces Android/Linux Wi-Fi TX even with 5G active)
    for iface in ifaces {
        if is_cgnat(iface.ip) {
            continue;
        }
        let tx_i = tx.clone();
        let bytes_i = probe_bytes.clone();
        let targets_i = vec![
            SocketAddr::from((iface.broadcast, discovery_port)),
            SocketAddr::from(([255, 255, 255, 255], discovery_port)),
        ];
        tokio::spawn(async move {
            if let Some(addr) = udp_probe_socket(iface.ip, targets_i, bytes_i, window).await {
                let _ = tx_i.send(addr).await;
            }
        });
    }
    drop(tx);

    tokio::time::timeout(window + Duration::from_millis(50), rx.recv())
        .await
        .ok()
        .flatten()
}

async fn verify_tcp_hub(
    addr: SocketAddr,
    identity: &ClientIdentity,
    device_id: &str,
    connect_timeout: Duration,
) -> HandshakeOutcome {
    let Ok(Ok(stream)) = tokio::time::timeout(connect_timeout, TcpStream::connect(addr)).await else {
        return HandshakeOutcome::Failed;
    };
    let _ = stream.set_nodelay(true);
    let (read_half, mut write_half) = tokio::io::split(stream);

    let Ok(line) = hello_frame(identity, device_id).to_json_line() else {
        return HandshakeOutcome::Failed;
    };
    if write_half.write_all(line.as_bytes()).await.is_err() || write_half.flush().await.is_err() {
        return HandshakeOutcome::Failed;
    }

    let mut lines = BufReader::new(read_half).lines();
    let Ok(Ok(Some(ack_line))) =
        tokio::time::timeout(Duration::from_millis(1500), lines.next_line()).await
    else {
        return HandshakeOutcome::Failed;
    };
    let Ok(msg) = serde_json::from_str::<HubDownlinkMessage>(ack_line.trim()) else {
        return HandshakeOutcome::Failed;
    };
    match msg {
        HubDownlinkMessage::HelloAck { .. } => {
            let read_half = lines.into_inner().into_inner();
            let stream = read_half.unsplit(write_half);
            HandshakeOutcome::Accepted(stream, msg)
        }
        HubDownlinkMessage::SlotFull { .. } => HandshakeOutcome::Rejected(msg),
        _ => HandshakeOutcome::Failed,
    }
}

enum DiscoveryResult {
    Connected(SocketAddr, TcpStream, HubDownlinkMessage),
    Rejected(HubDownlinkMessage),
    NoneFound,
}

/// Subnet `/24` TCP sweep fallback if Wi-Fi AP/hotspot isolates UDP broadcast traffic.
async fn discover_via_subnet_tcp(
    tcp_port: u16,
    identity: &ClientIdentity,
    device_id: &str,
) -> DiscoveryResult {
    let ifaces = detect_local_ipv4_ifaces();
    let has_private = ifaces.iter().any(|i| is_rfc1918(i.ip));

    for iface in ifaces {
        let ip = iface.ip;
        if has_private && !is_rfc1918(ip) {
            continue;
        }
        if is_cgnat(ip) {
            continue;
        }
        let oct = ip.octets();
        let mut candidates = Vec::with_capacity(253);
        for host in 1..=254_u8 {
            if host == oct[3] {
                continue;
            }
            candidates.push(SocketAddr::from(([oct[0], oct[1], oct[2], host], tcp_port)));
        }

        for chunk in candidates.chunks(64) {
            let mut handles = Vec::with_capacity(chunk.len());
            for &cand in chunk {
                let id_clone = identity.clone();
                let dev_clone = device_id.to_string();
                handles.push(tokio::spawn(async move {
                    (
                        cand,
                        verify_tcp_hub(cand, &id_clone, &dev_clone, Duration::from_millis(350))
                            .await,
                    )
                }));
            }
            for h in handles {
                if let Ok((cand, outcome)) = h.await {
                    match outcome {
                        HandshakeOutcome::Accepted(s, ack) => {
                            return DiscoveryResult::Connected(cand, s, ack);
                        }
                        HandshakeOutcome::Rejected(msg) => {
                            return DiscoveryResult::Rejected(msg);
                        }
                        HandshakeOutcome::Failed => {}
                    }
                }
            }
        }
    }
    DiscoveryResult::NoneFound
}

pub struct BiomonLanClient {
    endpoint_label: Arc<RwLock<String>>,
    state: Arc<Mutex<ConnectionState>>,
    manual_blackout: Arc<AtomicBool>,
    frames_sent: Arc<AtomicUsize>,
    next_batch_id: AtomicU64,
    identity: Arc<RwLock<ClientIdentity>>,
    rescan_notify: Arc<Notify>,
    outbound_tx: mpsc::UnboundedSender<OutboundFrame>,
    event_rx: mpsc::UnboundedReceiver<LinkEvent>,
    _runtime: tokio::runtime::Runtime,
}

impl BiomonLanClient {
    /// Connects cleanly to either a specified `host:port` or auto-discovers on the LAN (`"auto"` / `""`).
    pub fn new(target_hub_addr: impl Into<String>) -> Self {
        let target = target_hub_addr.into();
        let mut cfg = BiomonLanConfig::default();
        let trimmed = target.trim();
        if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("auto") {
            cfg.hub_override_addr = Some(trimmed.to_string());
        }
        Self::with_config(cfg)
    }

    pub fn new_auto(crew_id: &str, callsign: &str, active_phase: &str) -> Self {
        let cfg = BiomonLanConfig {
            crew_id: crew_id.to_string(),
            callsign: callsign.to_string(),
            active_phase: active_phase.to_string(),
            ..Default::default()
        };
        Self::with_config(cfg)
    }

    pub fn with_config(cfg: BiomonLanConfig) -> Self {
        let initial_label = cfg
            .hub_override_addr
            .clone()
            .unwrap_or_else(|| "SEARCHING LAN...".to_string());
        let endpoint_label = Arc::new(RwLock::new(initial_label));
        let state = Arc::new(Mutex::new(ConnectionState::Offline));
        let manual_blackout = Arc::new(AtomicBool::new(false));
        let frames_sent = Arc::new(AtomicUsize::new(0));
        let identity = Arc::new(RwLock::new(ClientIdentity {
            crew_id: cfg.crew_id.clone(),
            callsign: cfg.callsign.clone(),
            active_phase: cfg.active_phase.clone(),
        }));
        let rescan_notify = Arc::new(Notify::new());

        let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<OutboundFrame>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<LinkEvent>();

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to build BiomonLanClient tokio runtime");

        let label_clone = Arc::clone(&endpoint_label);
        let state_clone = Arc::clone(&state);
        let blackout_clone = Arc::clone(&manual_blackout);
        let sent_clone = Arc::clone(&frames_sent);
        let id_clone = Arc::clone(&identity);
        let notify_clone = Arc::clone(&rescan_notify);
        let device_id = next_device_id();

        runtime.spawn(async move {

            loop {
                // 1. Honor manual RF blackout simulation
                if blackout_clone.load(Ordering::SeqCst) {
                    if let Ok(mut g) = state_clone.lock() {
                        *g = ConnectionState::Offline;
                    }
                    if let Ok(mut lbl) = label_clone.write() {
                        *lbl = "BLACKOUT (MANUAL)".to_string();
                    }
                    notify_clone.notified().await;
                    continue;
                }

                // Drain stale buffered live frames; SQLite holds durable unsynced records.
                while outbound_rx.try_recv().is_ok() {}

                let cur_id = id_clone
                    .read()
                    .map(|g| g.clone())
                    .unwrap_or(ClientIdentity {
                        crew_id: "AST-01".to_string(),
                        callsign: "ARES-1".to_string(),
                        active_phase: "ORBIT".to_string(),
                    });

                // 2. Obtain verified TCP connection (direct override, UDP beacon, or /24 fallback)
                let outcome = if let Some(ref raw_override) = cfg.hub_override_addr {
                    if let Ok(mut lbl) = label_clone.write() {
                        *lbl = raw_override.clone();
                    }
                    if let Ok(addr) = raw_override.parse::<SocketAddr>() {
                        match verify_tcp_hub(addr, &cur_id, &device_id, Duration::from_millis(800))
                            .await
                        {
                            HandshakeOutcome::Accepted(s, ack) => {
                                DiscoveryResult::Connected(addr, s, ack)
                            }
                            HandshakeOutcome::Rejected(msg) => DiscoveryResult::Rejected(msg),
                            HandshakeOutcome::Failed => DiscoveryResult::NoneFound,
                        }
                    } else {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        DiscoveryResult::NoneFound
                    }
                } else {
                    let lan_hint = detect_local_ipv4s()
                        .into_iter()
                        .find(|ip| is_rfc1918(*ip))
                        .map(|ip| {
                            let o = ip.octets();
                            format!("SEARCHING {}.{}.{}.x", o[0], o[1], o[2])
                        })
                        .unwrap_or_else(|| "SEARCHING LAN...".to_string());
                    if let Ok(mut lbl) = label_clone.write() {
                        *lbl = lan_hint;
                    }
                    if let Some(hub_addr) = discover_via_udp(cfg.discovery_port, &cur_id).await {
                        match verify_tcp_hub(
                            hub_addr,
                            &cur_id,
                            &device_id,
                            Duration::from_millis(1500),
                        )
                        .await
                        {
                            HandshakeOutcome::Accepted(s, ack) => {
                                DiscoveryResult::Connected(hub_addr, s, ack)
                            }
                            HandshakeOutcome::Rejected(msg) => DiscoveryResult::Rejected(msg),
                            HandshakeOutcome::Failed => {
                                discover_via_subnet_tcp(cfg.tcp_fallback_port, &cur_id, &device_id)
                                    .await
                            }
                        }
                    } else {
                        discover_via_subnet_tcp(cfg.tcp_fallback_port, &cur_id, &device_id).await
                    }
                };

                let (hub_addr, stream, hello_ack) = match outcome {
                    DiscoveryResult::Connected(addr, stream, ack) => (addr, stream, ack),
                    DiscoveryResult::Rejected(slot_full) => {
                        if let Ok(mut lbl) = label_clone.write() {
                            *lbl = "ROSTER FULL (MAX 6)".to_string();
                        }
                        if let Ok(mut g) = state_clone.lock() {
                            *g = ConnectionState::Offline;
                        }
                        let _ = event_tx.send(LinkEvent::Downlink(slot_full));
                        tokio::select! {
                            _ = tokio::time::sleep(Duration::from_millis(1500)) => {}
                            _ = notify_clone.notified() => {}
                        }
                        continue;
                    }
                    DiscoveryResult::NoneFound => {
                        tokio::select! {
                            _ = tokio::time::sleep(Duration::from_millis(400)) => {}
                            _ = notify_clone.notified() => {}
                        }
                        continue;
                    }
                };

                if blackout_clone.load(Ordering::SeqCst) {
                    continue;
                }

                // Adopt Hub-assigned crew slot immediately so HEARTBEAT / telemetry carry the exact slot.
                if let HubDownlinkMessage::HelloAck {
                    ref assigned_crew_id,
                    ref assigned_callsign,
                    ..
                } = hello_ack
                    && !assigned_crew_id.is_empty()
                    && let Ok(mut id_g) = id_clone.write()
                {
                    id_g.crew_id = assigned_crew_id.clone();
                    if !assigned_callsign.is_empty() {
                        id_g.callsign = assigned_callsign.clone();
                    }
                }

                // 3. Connected & Verified!
                let addr_str = hub_addr.to_string();
                if let Ok(mut lbl) = label_clone.write() {
                    *lbl = addr_str.clone();
                }
                if let Ok(mut g) = state_clone.lock() {
                    *g = ConnectionState::Online;
                }
                let _ = event_tx.send(LinkEvent::Connected(addr_str));
                let _ = event_tx.send(LinkEvent::Downlink(hello_ack));

                let (read_half, mut write_half) = tokio::io::split(stream);
                let mut reader = BufReader::new(read_half).lines();
                let mut heartbeat = tokio::time::interval(Duration::from_millis(2500));
                heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

                loop {
                    tokio::select! {
                        _ = notify_clone.notified() => {
                            // Identity changed, manual blackout toggled, or explicit rescan requested
                            break;
                        }
                        maybe_frame = outbound_rx.recv() => {
                            let Some(frame) = maybe_frame else {
                                return;
                            };
                            if blackout_clone.load(Ordering::SeqCst) {
                                break;
                            }
                            if let Ok(line) = frame.to_json_line() {
                                if write_half.write_all(line.as_bytes()).await.is_err()
                                    || write_half.flush().await.is_err()
                                {
                                    break;
                                }
                                sent_clone.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        maybe_line = reader.next_line() => {
                            match maybe_line {
                                Ok(Some(line)) => {
                                    let trimmed = line.trim();
                                    if !trimmed.is_empty()
                                        && let Ok(msg) = serde_json::from_str::<HubDownlinkMessage>(trimmed)
                                    {
                                        let _ = event_tx.send(LinkEvent::Downlink(msg));
                                    }
                                }
                                _ => break,
                            }
                        }
                        _ = heartbeat.tick() => {
                            if blackout_clone.load(Ordering::SeqCst) {
                                break;
                            }
                            let cur = id_clone
                                .read()
                                .map(|g| g.clone())
                                .unwrap_or(ClientIdentity {
                                    crew_id: "AST-01".to_string(),
                                    callsign: "ARES-1".to_string(),
                                    active_phase: "ORBIT".to_string(),
                                });
                            if let Ok(line) = heartbeat_frame(&cur).to_json_line()
                                && (write_half.write_all(line.as_bytes()).await.is_err()
                                    || write_half.flush().await.is_err())
                            {
                                break;
                            }
                        }
                    }
                }

                // 4. Dropped or closed -> back to offline / discovery
                if let Ok(mut g) = state_clone.lock() {
                    *g = ConnectionState::Offline;
                }
                let _ = event_tx.send(LinkEvent::Disconnected);
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });

        Self {
            endpoint_label,
            state,
            manual_blackout,
            frames_sent,
            next_batch_id: AtomicU64::new(1),
            identity,
            rescan_notify,
            outbound_tx,
            event_rx,
            _runtime: runtime,
        }
    }

    pub fn target_hub_addr(&self) -> String {
        self.endpoint_label
            .read()
            .map(|g| g.clone())
            .unwrap_or_else(|_| "SEARCHING LAN...".to_string())
    }

    pub fn connection_state(&self) -> ConnectionState {
        if self.manual_blackout.load(Ordering::SeqCst) {
            return ConnectionState::Offline;
        }
        self.state
            .lock()
            .map(|g| *g)
            .unwrap_or(ConnectionState::Offline)
    }

    pub fn is_manual_blackout(&self) -> bool {
        self.manual_blackout.load(Ordering::SeqCst)
    }

    pub fn toggle_connection_state(&self) -> ConnectionState {
        let was_blackout = self.manual_blackout.fetch_xor(true, Ordering::SeqCst);
        let now_blackout = !was_blackout;
        if now_blackout && let Ok(mut g) = self.state.lock() {
            *g = ConnectionState::Offline;
        }
        self.rescan_notify.notify_waiters();
        self.connection_state()
    }

    pub fn trigger_rescan(&self) {
        self.manual_blackout.store(false, Ordering::SeqCst);
        self.rescan_notify.notify_waiters();
    }

    pub fn set_identity(&self, crew_id: &str, callsign: &str, active_phase: &str) {
        if let Ok(mut g) = self.identity.write() {
            g.crew_id = crew_id.to_string();
            g.callsign = callsign.to_string();
            g.active_phase = active_phase.to_string();
        }
        self.rescan_notify.notify_waiters();
    }

    pub fn next_batch_id(&self) -> u64 {
        self.next_batch_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn frames_sent(&self) -> usize {
        self.frames_sent.load(Ordering::Relaxed)
    }

    pub fn crew_id(&self) -> String {
        self.identity
            .read()
            .map(|g| g.crew_id.clone())
            .unwrap_or_else(|_| "AST-01".to_string())
    }

    pub fn queue_frame(&self, frame: OutboundFrame) -> bool {
        if self.connection_state() != ConnectionState::Online {
            return false;
        }
        self.outbound_tx.send(frame).is_ok()
    }

    pub fn try_recv_event(&mut self) -> Option<LinkEvent> {
        self.event_rx.try_recv().ok()
    }
}
