use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket as StdUdpSocket};
use std::sync::{
    Arc, Mutex, RwLock,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::{Notify, mpsc};

pub const DEFAULT_TCP_PORT: u16 = 7878;
pub const DEFAULT_DISCOVERY_PORT: u16 = 7879;
pub const DISCOVERY_MAGIC_REQ: &str = "AEGIS_HUB_DISCOVER";
pub const DISCOVERY_MAGIC_RESP: &str = "AEGIS_HUB_BEACON";

static NEXT_CLIENT_SEQ: AtomicU64 = AtomicU64::new(1);

fn next_device_id() -> String {
    let seq = NEXT_CLIENT_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("dev-{}-{}-{}", std::process::id(), seq, nanos)
}

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    #[serde(skip_serializing, default)]
    pub sync_batch: Option<SyncBatch>,
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
    prefix_len: u8,
}

fn is_rfc1918(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    (o[0] == 192 && o[1] == 168) || (o[0] == 172 && (16..=31).contains(&o[1])) || o[0] == 10
}

fn is_cgnat(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 100 && (64..=127).contains(&o[1])
}

fn is_windows_hotspot_subnet(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 192 && o[1] == 168 && o[2] == 137
}

fn normalize_hub_override(raw: &str, default_port: u16) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        return None;
    }
    if trimmed.parse::<SocketAddr>().is_ok() {
        return Some(trimmed.to_string());
    }
    if let Ok(v4) = trimmed.parse::<Ipv4Addr>() {
        return Some(format!("{v4}:{default_port}"));
    }
    Some(trimmed.to_string())
}

fn push_iface(
    out: &mut Vec<LocalIpv4Iface>,
    ip: Ipv4Addr,
    broadcast: Option<Ipv4Addr>,
    prefix_len: u8,
) {
    if ip.is_unspecified() || ip.is_loopback() || ip.is_link_local() || ip.is_multicast() {
        return;
    }
    let bcast = broadcast.unwrap_or_else(|| {
        let o = ip.octets();
        Ipv4Addr::new(o[0], o[1], o[2], 255)
    });
    if !out.iter().any(|x| x.ip == ip) {
        out.push(LocalIpv4Iface {
            ip,
            broadcast: bcast,
            prefix_len: prefix_len.clamp(8, 32),
        });
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
            if is_up
                && !is_loop
                && !ifa.ifa_addr.is_null()
                && (*ifa.ifa_addr).sa_family as i32 == libc::AF_INET
            {
                let sa = &*(ifa.ifa_addr as *const libc::sockaddr_in);
                let ip = Ipv4Addr::from(u32::from_be(sa.sin_addr.s_addr));
                let (bcast, prefix_len) = if !ifa.ifa_netmask.is_null()
                    && (*ifa.ifa_netmask).sa_family as i32 == libc::AF_INET
                {
                    let nm = &*(ifa.ifa_netmask as *const libc::sockaddr_in);
                    let mask_u32 = u32::from_be(nm.sin_addr.s_addr);
                    let ip_u32 = u32::from_be(sa.sin_addr.s_addr);
                    if mask_u32 != 0 {
                        (
                            Some(Ipv4Addr::from((ip_u32 & mask_u32) | !mask_u32)),
                            (mask_u32.count_ones() as u8).clamp(8, 32),
                        )
                    } else {
                        (None, 24)
                    }
                } else {
                    (None, 24)
                };
                push_iface(out, ip, bcast, prefix_len);
            }
            cur = ifa.ifa_next;
        }
        libc::freeifaddrs(ifap);
    }
}

#[cfg(not(unix))]
fn collect_unix_ifaces(_out: &mut Vec<LocalIpv4Iface>) {}

/// Enumerates active local IPv4 interfaces (Android/Linux `getifaddrs` + multi-subnet OS route probes),
/// sorted with laptop hotspot (`192.168.137.x`) and RFC-1918 LAN interfaces first.
fn detect_local_ipv4_ifaces() -> Vec<LocalIpv4Iface> {
    let mut found = Vec::new();
    collect_unix_ifaces(&mut found);

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
        {
            push_iface(&mut found, v4, None, 24);
        }
    }

    found.sort_by_key(|iface| {
        if is_windows_hotspot_subnet(iface.ip) {
            0_u8
        } else if is_rfc1918(iface.ip) {
            1_u8
        } else if !is_cgnat(iface.ip) {
            2_u8
        } else {
            3_u8
        }
    });
    found
}

pub fn detect_local_ipv4s() -> Vec<Ipv4Addr> {
    detect_local_ipv4_ifaces()
        .into_iter()
        .map(|i| i.ip)
        .collect()
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
        sync_batch: None,
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
        sync_batch: None,
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
        if let Some(addr) =
            udp_probe_socket(Ipv4Addr::UNSPECIFIED, default_targets, bytes0, window).await
        {
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
    let Ok(Ok(stream)) = tokio::time::timeout(connect_timeout, TcpStream::connect(addr)).await
    else {
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

/// Fast-path probe directly against `.1` gateway of each local non-CGNAT RFC-1918 interface
/// (e.g. `192.168.137.1` when connected to a Windows Laptop Hotspot), avoiding full subnet sweep.
async fn probe_gateway_tcp(
    tcp_port: u16,
    identity: &ClientIdentity,
    device_id: &str,
) -> DiscoveryResult {
    let ifaces = detect_local_ipv4_ifaces();
    let mut gateways = Vec::new();
    for iface in ifaces {
        if !is_rfc1918(iface.ip) || is_cgnat(iface.ip) {
            continue;
        }
        let o = iface.ip.octets();
        let gw = Ipv4Addr::new(o[0], o[1], o[2], 1);
        if iface.ip != gw {
            let sa = SocketAddr::from((gw, tcp_port));
            if !gateways.contains(&sa) {
                gateways.push(sa);
            }
        }
    }
    if gateways.is_empty() {
        return DiscoveryResult::NoneFound;
    }

    let mut handles = Vec::with_capacity(gateways.len());
    for gw in gateways {
        let id_clone = identity.clone();
        let dev_clone = device_id.to_string();
        handles.push(tokio::spawn(async move {
            (
                gw,
                verify_tcp_hub(gw, &id_clone, &dev_clone, Duration::from_millis(350)).await,
            )
        }));
    }
    for h in handles {
        if let Ok((addr, outcome)) = h.await {
            match outcome {
                HandshakeOutcome::Accepted(s, ack) => {
                    return DiscoveryResult::Connected(addr, s, ack);
                }
                HandshakeOutcome::Rejected(msg) => {
                    return DiscoveryResult::Rejected(msg);
                }
                HandshakeOutcome::Failed => {}
            }
        }
    }
    DiscoveryResult::NoneFound
}

async fn discover_udp_or_gateway(
    discovery_port: u16,
    tcp_port: u16,
    identity: &ClientIdentity,
    device_id: &str,
) -> DiscoveryResult {
    let id_udp = identity.clone();
    let dev_udp = device_id.to_string();
    let udp_task = tokio::spawn(async move {
        if let Some(hub_addr) = discover_via_udp(discovery_port, &id_udp).await {
            match verify_tcp_hub(hub_addr, &id_udp, &dev_udp, Duration::from_millis(1200)).await {
                HandshakeOutcome::Accepted(s, ack) => DiscoveryResult::Connected(hub_addr, s, ack),
                HandshakeOutcome::Rejected(msg) => DiscoveryResult::Rejected(msg),
                HandshakeOutcome::Failed => DiscoveryResult::NoneFound,
            }
        } else {
            DiscoveryResult::NoneFound
        }
    });

    let id_gw = identity.clone();
    let dev_gw = device_id.to_string();
    let gw_task = tokio::spawn(async move { probe_gateway_tcp(tcp_port, &id_gw, &dev_gw).await });

    tokio::pin!(udp_task);
    tokio::pin!(gw_task);

    tokio::select! {
        res = &mut udp_task => {
            match res.unwrap_or(DiscoveryResult::NoneFound) {
                found @ (DiscoveryResult::Connected(..) | DiscoveryResult::Rejected(..)) => found,
                DiscoveryResult::NoneFound => gw_task.await.unwrap_or(DiscoveryResult::NoneFound),
            }
        }
        res = &mut gw_task => {
            match res.unwrap_or(DiscoveryResult::NoneFound) {
                found @ (DiscoveryResult::Connected(..) | DiscoveryResult::Rejected(..)) => found,
                DiscoveryResult::NoneFound => udp_task.await.unwrap_or(DiscoveryResult::NoneFound),
            }
        }
    }
}

/// Subnet TCP sweep fallback if UDP broadcast is dropped and Hub is not at `.1`.
/// For wider subnets (`prefix_len < 24`, e.g. `/22` gov/campus Wi-Fi), sweeps up to +-2 adjacent `/24` blocks.
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
        let mut slash24_blocks: Vec<[u8; 3]> = vec![[oct[0], oct[1], oct[2]]];
        if iface.prefix_len < 24 && iface.prefix_len >= 16 {
            let ip_u32 = u32::from(ip);
            let mask = !0_u32 << (32 - iface.prefix_len);
            let net_base = ip_u32 & mask;
            let net_end = net_base | !mask;
            let my_block = (ip_u32 >> 8) as i64;
            let min_block = (net_base >> 8) as i64;
            let max_block = (net_end >> 8) as i64;
            for delta in [1_i64, -1, 2, -2] {
                let b = my_block + delta;
                if b >= min_block && b <= max_block {
                    slash24_blocks.push([
                        ((b >> 16) & 0xff) as u8,
                        ((b >> 8) & 0xff) as u8,
                        (b & 0xff) as u8,
                    ]);
                }
            }
        }

        for blk in slash24_blocks {
            let mut candidates = Vec::with_capacity(254);
            for host in 1..=254_u8 {
                if blk == [oct[0], oct[1], oct[2]] && host == oct[3] {
                    continue;
                }
                candidates.push(SocketAddr::from(([blk[0], blk[1], blk[2], host], tcp_port)));
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
    }
    DiscoveryResult::NoneFound
}

pub struct BiomonLanClient {
    endpoint_label: Arc<RwLock<String>>,
    hub_override: Arc<RwLock<Option<String>>>,
    default_tcp_port: u16,
    state: Arc<Mutex<ConnectionState>>,
    manual_blackout: Arc<AtomicBool>,
    hub_reachable: Arc<AtomicBool>,
    frames_sent: Arc<AtomicUsize>,
    next_batch_id: AtomicU64,
    identity: Arc<RwLock<ClientIdentity>>,
    rescan_notify: Arc<Notify>,
    outbound_tx: mpsc::UnboundedSender<OutboundFrame>,
    ack_tx: mpsc::UnboundedSender<AckEvent>,
    ack_rx: Mutex<mpsc::UnboundedReceiver<AckEvent>>,
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
        Self::new_auto_with_override(crew_id, callsign, active_phase, None)
    }

    pub fn new_auto_with_override(
        crew_id: &str,
        callsign: &str,
        active_phase: &str,
        persisted_override: Option<String>,
    ) -> Self {
        let mut cfg = BiomonLanConfig {
            crew_id: crew_id.to_string(),
            callsign: callsign.to_string(),
            active_phase: active_phase.to_string(),
            ..Default::default()
        };
        if cfg.hub_override_addr.is_none()
            && let Some(raw) = persisted_override
        {
            cfg.hub_override_addr = normalize_hub_override(&raw, cfg.tcp_fallback_port);
        }
        Self::with_config(cfg)
    }

    pub fn with_config(cfg: BiomonLanConfig) -> Self {
        let normalized_override = cfg
            .hub_override_addr
            .as_deref()
            .and_then(|s| normalize_hub_override(s, cfg.tcp_fallback_port));
        let default_tcp_port = cfg.tcp_fallback_port;
        let initial_label = normalized_override
            .clone()
            .unwrap_or_else(|| "SEARCHING LAN...".to_string());
        let endpoint_label = Arc::new(RwLock::new(initial_label));
        let hub_override = Arc::new(RwLock::new(normalized_override));
        let state = Arc::new(Mutex::new(ConnectionState::Offline));
        let manual_blackout = Arc::new(AtomicBool::new(false));
        let hub_reachable = Arc::new(AtomicBool::new(false));
        let frames_sent = Arc::new(AtomicUsize::new(0));
        let identity = Arc::new(RwLock::new(ClientIdentity {
            crew_id: cfg.crew_id.clone(),
            callsign: cfg.callsign.clone(),
            active_phase: cfg.active_phase.clone(),
        }));
        let rescan_notify = Arc::new(Notify::new());

        let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<OutboundFrame>();
        let (ack_tx, ack_rx) = mpsc::unbounded_channel::<AckEvent>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<LinkEvent>();

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to build BiomonLanClient tokio runtime");

        let label_clone = Arc::clone(&endpoint_label);
        let override_clone = Arc::clone(&hub_override);
        let state_clone = Arc::clone(&state);
        let blackout_clone = Arc::clone(&manual_blackout);
        let reachable_clone = Arc::clone(&hub_reachable);
        let sent_clone = Arc::clone(&frames_sent);
        let id_clone = Arc::clone(&identity);
        let notify_clone = Arc::clone(&rescan_notify);
        let ack_worker_tx = ack_tx.clone();
        let device_id = next_device_id();

        runtime.spawn(async move {
            loop {
                // 1. Honor manual RF blackout simulation
                if blackout_clone.load(Ordering::SeqCst) {
                    reachable_clone.store(false, Ordering::SeqCst);
                    if let Ok(mut g) = state_clone.lock() {
                        *g = ConnectionState::Offline;
                    }
                    if let Ok(mut lbl) = label_clone.write() {
                        *lbl = "BLACKOUT (MANUAL)".to_string();
                    }
                    notify_clone.notified().await;
                    continue;
                }

                // Drain any pending frames left over from a dropped socket and report negative ACKs
                // so in-flight WAL flags reset cleanly without losing SQLite durability.
                while let Ok(stale) = outbound_rx.try_recv() {
                    let _ = ack_worker_tx.send(AckEvent {
                        delivered: false,
                        packet_type: stale.packet_type,
                        sync_batch: stale.sync_batch,
                        error: Some("HUB UNREACHABLE".to_string()),
                    });
                }

                let cur_id = id_clone
                    .read()
                    .map(|g| g.clone())
                    .unwrap_or(ClientIdentity {
                        crew_id: "AST-01".to_string(),
                        callsign: "ARES-1".to_string(),
                        active_phase: "ORBIT".to_string(),
                    });

                // 2. Obtain verified TCP connection (direct override, UDP+Gateway race, or subnet sweep)
                let active_override = override_clone.read().ok().and_then(|g| g.clone());
                let outcome = if let Some(ref raw_override) = active_override {
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
                        if let Ok(mut lbl) = label_clone.write() {
                            *lbl = format!("INVALID IP: {raw_override}");
                        }
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
                    match discover_udp_or_gateway(
                        cfg.discovery_port,
                        cfg.tcp_fallback_port,
                        &cur_id,
                        &device_id,
                    )
                    .await
                    {
                        found @ (DiscoveryResult::Connected(..) | DiscoveryResult::Rejected(..)) => {
                            found
                        }
                        DiscoveryResult::NoneFound => {
                            discover_via_subnet_tcp(cfg.tcp_fallback_port, &cur_id, &device_id)
                                .await
                        }
                    }
                };

                let (hub_addr, stream, hello_ack) = match outcome {
                    DiscoveryResult::Connected(addr, stream, ack) => (addr, stream, ack),
                    DiscoveryResult::Rejected(slot_full) => {
                        reachable_clone.store(false, Ordering::SeqCst);
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
                        reachable_clone.store(false, Ordering::SeqCst);
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
                reachable_clone.store(true, Ordering::SeqCst);
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
                let mut heartbeat = tokio::time::interval_at(
                    tokio::time::Instant::now() + Duration::from_millis(2500),
                    Duration::from_millis(2500),
                );
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
                                let _ = ack_worker_tx.send(AckEvent {
                                    delivered: false,
                                    packet_type: frame.packet_type,
                                    sync_batch: frame.sync_batch,
                                    error: Some("MANUAL BLACKOUT".to_string()),
                                });
                                break;
                            }
                            if let Ok(line) = frame.to_json_line() {
                                if write_half.write_all(line.as_bytes()).await.is_err()
                                    || write_half.flush().await.is_err()
                                {
                                    let _ = ack_worker_tx.send(AckEvent {
                                        delivered: false,
                                        packet_type: frame.packet_type,
                                        sync_batch: frame.sync_batch,
                                        error: Some("TCP WRITE FAILED".to_string()),
                                    });
                                    break;
                                }
                                sent_clone.fetch_add(1, Ordering::Relaxed);
                                let _ = ack_worker_tx.send(AckEvent {
                                    delivered: true,
                                    packet_type: frame.packet_type,
                                    sync_batch: frame.sync_batch,
                                    error: None,
                                });
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
                reachable_clone.store(false, Ordering::SeqCst);
                if let Ok(mut g) = state_clone.lock() {
                    *g = ConnectionState::Offline;
                }
                let _ = event_tx.send(LinkEvent::Disconnected);
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });

        Self {
            endpoint_label,
            hub_override,
            default_tcp_port,
            state,
            manual_blackout,
            hub_reachable,
            frames_sent,
            next_batch_id: AtomicU64::new(1),
            identity,
            rescan_notify,
            outbound_tx,
            ack_tx,
            ack_rx: Mutex::new(ack_rx),
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

    pub fn is_hub_reachable(&self) -> bool {
        self.hub_reachable.load(Ordering::SeqCst)
    }

    pub fn toggle_connection_state(&self) -> ConnectionState {
        let was_blackout = self.manual_blackout.fetch_xor(true, Ordering::SeqCst);
        let now_blackout = !was_blackout;
        if now_blackout {
            self.hub_reachable.store(false, Ordering::SeqCst);
            if let Ok(mut g) = self.state.lock() {
                *g = ConnectionState::Offline;
            }
            if let Ok(mut lbl) = self.endpoint_label.write() {
                *lbl = "BLACKOUT (MANUAL)".to_string();
            }
        }
        self.rescan_notify.notify_waiters();
        self.connection_state()
    }

    pub fn trigger_rescan(&self) {
        self.manual_blackout.store(false, Ordering::SeqCst);
        self.rescan_notify.notify_waiters();
    }

    pub fn trigger_probe(&self) {
        self.trigger_rescan();
    }

    pub fn hub_override(&self) -> Option<String> {
        self.hub_override.read().ok().and_then(|g| g.clone())
    }

    pub fn set_hub_override(&self, raw: Option<&str>) {
        let normalized = raw.and_then(|s| normalize_hub_override(s, self.default_tcp_port));
        if let Ok(mut ov) = self.hub_override.write() {
            *ov = normalized.clone();
        }
        if let Ok(mut lbl) = self.endpoint_label.write() {
            *lbl = normalized.unwrap_or_else(|| "SEARCHING LAN...".to_string());
        }
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
            let _ = self.ack_tx.send(AckEvent {
                delivered: false,
                packet_type: frame.packet_type,
                sync_batch: frame.sync_batch,
                error: Some(if self.is_manual_blackout() {
                    "MANUAL BLACKOUT".to_string()
                } else {
                    "HUB UNREACHABLE".to_string()
                }),
            });
            return false;
        }
        self.outbound_tx.send(frame).is_ok()
    }

    pub fn try_recv_ack(&self) -> Option<AckEvent> {
        self.ack_rx.lock().ok()?.try_recv().ok()
    }

    pub fn try_recv_event(&mut self) -> Option<LinkEvent> {
        self.event_rx.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
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
                let (r, mut w) = tokio::io::split(sock);
                let mut lines = BufReader::new(r).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line)
                        && v["packet_type"] == "HELLO"
                    {
                        let ack = HubDownlinkMessage::HelloAck {
                            hub_id: "TEST-HUB".to_string(),
                            mission_phase: "ORBIT".to_string(),
                            distance_au: 1.32,
                            one_way_light_min: 11.0,
                            assigned_crew_id: "AST-01".to_string(),
                            assigned_callsign: "ARES-1".to_string(),
                        };
                        let s = serde_json::to_string(&ack).unwrap() + "\n";
                        let _ = w.write_all(s.as_bytes()).await;
                        let _ = w.flush().await;
                    } else if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line)
                        && v["packet_type"] == "HEARTBEAT"
                    {
                        // Ignore periodic keepalive heartbeats in delivery assertion
                    } else {
                        let _ = received_tx.send(line);
                    }
                }
            }
        });

        let client = BiomonLanClient::new(addr.to_string());
        for _ in 0..80 {
            if client.connection_state() == ConnectionState::Online {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        assert_eq!(client.connection_state(), ConnectionState::Online);

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
        assert!(!client.queue_frame(OutboundFrame {
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

    #[test]
    fn test_runtime_hub_override_switch() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.spawn(async move {
            let tokio_listener = TcpListener::from_std(listener).unwrap();
            if let Ok((sock, _)) = tokio_listener.accept().await {
                let (r, mut w) = tokio::io::split(sock);
                let mut lines = BufReader::new(r).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line)
                        && v["packet_type"] == "HELLO"
                    {
                        let ack = HubDownlinkMessage::HelloAck {
                            hub_id: "HOTSPOT-HUB".to_string(),
                            mission_phase: "ORBIT".to_string(),
                            distance_au: 1.32,
                            one_way_light_min: 11.0,
                            assigned_crew_id: "AST-02".to_string(),
                            assigned_callsign: "ARES-2".to_string(),
                        };
                        let s = serde_json::to_string(&ack).unwrap() + "\n";
                        let _ = w.write_all(s.as_bytes()).await;
                        let _ = w.flush().await;
                    }
                }
            }
        });

        let client = BiomonLanClient::new("127.0.0.1:0");
        assert_eq!(client.connection_state(), ConnectionState::Offline);

        client.set_hub_override(Some(&addr.ip().to_string()));
        assert_eq!(
            client.hub_override().as_deref(),
            Some(format!("127.0.0.1:{DEFAULT_TCP_PORT}").as_str())
        );

        client.set_hub_override(Some(&addr.to_string()));
        assert_eq!(
            client.hub_override().as_deref(),
            Some(addr.to_string().as_str())
        );

        for _ in 0..80 {
            if client.connection_state() == ConnectionState::Online {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        assert_eq!(client.connection_state(), ConnectionState::Online);
        assert_eq!(client.crew_id(), "AST-02");

        client.set_hub_override(None);
        assert!(client.hub_override().is_none());
    }
}
