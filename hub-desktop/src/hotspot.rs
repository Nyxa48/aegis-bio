use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub const DEFAULT_HOTSPOT_SSID: &str = "AEGIS-HUB";
pub const DEFAULT_HOTSPOT_PASS: &str = "aegisbio2026";
pub const FIREWALL_RULE_TCP: &str = "AEGIS-BIO Hub TCP";
pub const FIREWALL_RULE_UDP: &str = "AEGIS-BIO Hub UDP";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotspotMode {
    Off,
    MobileHotspot,
    WifiDirectAp,
    Transitioning,
}

impl HotspotMode {
    pub fn label(self) -> &'static str {
        match self {
            HotspotMode::Off => "OFF",
            HotspotMode::MobileHotspot => "MOBILE HOTSPOT",
            HotspotMode::WifiDirectAp => "OFFLINE SOFTAP",
            HotspotMode::Transitioning => "STARTING...",
        }
    }
}

#[derive(Debug, Clone)]
pub struct HotspotSnapshot {
    pub mode: HotspotMode,
    pub active: bool,
    pub transitioning: bool,
    pub ssid: String,
    pub passphrase: String,
    pub gateway_ip: Option<String>,
    pub client_count: u32,
    pub max_clients: u32,
    pub firewall_rule_ok: bool,
    pub status_text: String,
    pub last_error: Option<String>,
}

impl Default for HotspotSnapshot {
    fn default() -> Self {
        Self {
            mode: HotspotMode::Off,
            active: false,
            transitioning: false,
            ssid: DEFAULT_HOTSPOT_SSID.to_string(),
            passphrase: DEFAULT_HOTSPOT_PASS.to_string(),
            gateway_ip: None,
            client_count: 0,
            max_clients: 8,
            firewall_rule_ok: false,
            status_text: "HOTSPOT STANDBY".to_string(),
            last_error: None,
        }
    }
}

enum HotspotCmd {
    Refresh,
    Start,
    Stop,
    ApplyAegisCredentials,
    EnsureFirewall,
    Shutdown,
}

pub struct HotspotManager {
    snapshot: Arc<RwLock<HotspotSnapshot>>,
    tx: Sender<HotspotCmd>,
}

impl HotspotManager {
    pub fn new() -> Self {
        let snapshot = Arc::new(RwLock::new(HotspotSnapshot::default()));
        let (tx, rx) = mpsc::channel::<HotspotCmd>();

        let snap_clone = Arc::clone(&snapshot);
        std::thread::spawn(move || {
            platform::worker_loop(rx, snap_clone);
        });

        Self { snapshot, tx }
    }

    pub fn snapshot(&self) -> HotspotSnapshot {
        self.snapshot.read().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn toggle(&self) {
        let active = self.snapshot().active;
        if active {
            self.stop();
        } else {
            self.start();
        }
    }

    pub fn start(&self) {
        if let Ok(mut g) = self.snapshot.write() {
            g.transitioning = true;
            g.mode = HotspotMode::Transitioning;
            g.status_text = "STARTING LAPTOP HOTSPOT...".to_string();
            g.last_error = None;
        }
        let _ = self.tx.send(HotspotCmd::Start);
    }

    pub fn stop(&self) {
        if let Ok(mut g) = self.snapshot.write() {
            g.transitioning = true;
            g.status_text = "STOPPING HOTSPOT...".to_string();
        }
        let _ = self.tx.send(HotspotCmd::Stop);
    }

    pub fn apply_aegis_credentials(&self) {
        if let Ok(mut g) = self.snapshot.write() {
            g.status_text = "CONFIGURING SSID AEGIS-HUB...".to_string();
            g.last_error = None;
        }
        let _ = self.tx.send(HotspotCmd::ApplyAegisCredentials);
    }

    pub fn ensure_firewall_rules(&self) {
        if let Ok(mut g) = self.snapshot.write() {
            g.status_text = "OPENING WINDOWS FIREWALL (TCP 7878 / UDP 7879)...".to_string();
            g.last_error = None;
        }
        let _ = self.tx.send(HotspotCmd::EnsureFirewall);
    }

    pub fn refresh_now(&self) {
        let _ = self.tx.send(HotspotCmd::Refresh);
    }
}

impl Default for HotspotManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for HotspotManager {
    fn drop(&mut self) {
        let _ = self.tx.send(HotspotCmd::Shutdown);
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use windows::Devices::WiFiDirect::{
        WiFiDirectAdvertisementListenStateDiscoverability, WiFiDirectAdvertisementPublisher,
        WiFiDirectAdvertisementPublisherStatus, WiFiDirectConnectionListener,
    };
    use windows::Networking::Connectivity::{
        ConnectionProfile, NetworkConnectivityLevel, NetworkInformation,
    };
    use windows::Networking::NetworkOperators::{
        NetworkOperatorTetheringManager, TetheringOperationStatus, TetheringOperationalState,
    };
    use windows::Security::Credentials::PasswordCredential;
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
    use windows::core::HSTRING;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    struct WfdHold {
        publisher: WiFiDirectAdvertisementPublisher,
        _listener: Option<WiFiDirectConnectionListener>,
        ssid: String,
        passphrase: String,
    }

    pub(super) fn worker_loop(
        rx: mpsc::Receiver<HotspotCmd>,
        snapshot: Arc<RwLock<HotspotSnapshot>>,
    ) {
        let com_ok = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).is_ok() };
        let mut wfd: Option<WfdHold> = None;

        refresh_state(&snapshot, &wfd);

        loop {
            match rx.recv_timeout(Duration::from_secs(3)) {
                Ok(HotspotCmd::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    if let Some(ref hold) = wfd {
                        let _ = hold.publisher.Stop();
                    }
                    break;
                }
                Ok(HotspotCmd::Refresh) | Err(mpsc::RecvTimeoutError::Timeout) => {
                    refresh_state(&snapshot, &wfd);
                }
                Ok(HotspotCmd::Start) => {
                    start_hotspot(&snapshot, &mut wfd);
                    refresh_state(&snapshot, &wfd);
                }
                Ok(HotspotCmd::Stop) => {
                    stop_hotspot(&snapshot, &mut wfd);
                    refresh_state(&snapshot, &wfd);
                }
                Ok(HotspotCmd::ApplyAegisCredentials) => {
                    configure_aegis_credentials(&snapshot, &mut wfd);
                    refresh_state(&snapshot, &wfd);
                }
                Ok(HotspotCmd::EnsureFirewall) => {
                    ensure_firewall(&snapshot);
                    refresh_state(&snapshot, &wfd);
                }
            }
        }

        if com_ok {
            unsafe { CoUninitialize() };
        }
    }

    fn find_tethering_profile() -> Option<ConnectionProfile> {
        if let Ok(prof) = NetworkInformation::GetInternetConnectionProfile() {
            return Some(prof);
        }
        if let Ok(list) = NetworkInformation::GetConnectionProfiles() {
            for idx in 0..list.Size().unwrap_or(0) {
                if let Ok(prof) = list.GetAt(idx) {
                    let level = prof
                        .GetNetworkConnectivityLevel()
                        .unwrap_or(NetworkConnectivityLevel::None);
                    if level != NetworkConnectivityLevel::None {
                        return Some(prof);
                    }
                }
            }
        }
        None
    }

    fn create_tethering_manager() -> Result<NetworkOperatorTetheringManager, String> {
        let prof = find_tethering_profile().ok_or_else(|| {
            "No active upstream network profile for Windows Mobile Hotspot".to_string()
        })?;
        NetworkOperatorTetheringManager::CreateFromConnectionProfile(&prof)
            .map_err(|e| format!("Tethering manager unavailable: {e}"))
    }

    fn check_firewall_rule_exists() -> bool {
        let tcp_ok = Command::new("netsh")
            .args([
                "advfirewall",
                "firewall",
                "show",
                "rule",
                &format!("name={FIREWALL_RULE_TCP}"),
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .is_ok_and(|o| o.status.success());

        let udp_ok = Command::new("netsh")
            .args([
                "advfirewall",
                "firewall",
                "show",
                "rule",
                &format!("name={FIREWALL_RULE_UDP}"),
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .is_ok_and(|o| o.status.success());

        tcp_ok && udp_ok
    }

    fn ensure_firewall(snapshot: &Arc<RwLock<HotspotSnapshot>>) {
        let cmd_script = format!(
            r#"netsh advfirewall firewall delete rule name="{tcp}" >nul 2>&1 & netsh advfirewall firewall delete rule name="{udp}" >nul 2>&1 & netsh advfirewall firewall add rule name="{tcp}" dir=in action=allow protocol=TCP localport=7878 profile=any & netsh advfirewall firewall add rule name="{udp}" dir=in action=allow protocol=UDP localport=7879 profile=any"#,
            tcp = FIREWALL_RULE_TCP,
            udp = FIREWALL_RULE_UDP
        );

        let ps_arg = format!(
            "Start-Process -FilePath 'cmd.exe' -ArgumentList '/c {cmd_script}' -Verb RunAs -WindowStyle Hidden -Wait"
        );

        match Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_arg])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
        {
            Ok(st) if st.success() => {
                let ok = check_firewall_rule_exists();
                if let Ok(mut g) = snapshot.write() {
                    g.firewall_rule_ok = ok;
                    if ok {
                        g.status_text = "FIREWALL PORTS OPENED (TCP 7878 / UDP 7879)".to_string();
                        g.last_error = None;
                    } else {
                        g.status_text = "FIREWALL RULE NOT CONFIRMED".to_string();
                        g.last_error = Some("UAC cancelled or firewall rule rejected".to_string());
                    }
                }
            }
            Ok(st) => {
                if let Ok(mut g) = snapshot.write() {
                    g.last_error = Some(format!("Firewall elevation exited with {st}"));
                }
            }
            Err(e) => {
                if let Ok(mut g) = snapshot.write() {
                    g.last_error = Some(format!("Failed to launch firewall helper: {e}"));
                }
            }
        }
    }

    fn configure_aegis_credentials(
        snapshot: &Arc<RwLock<HotspotSnapshot>>,
        wfd: &mut Option<WfdHold>,
    ) {
        if let Ok(tm) = create_tethering_manager()
            && let Ok(cfg) = tm.GetCurrentAccessPointConfiguration()
        {
            let _ = cfg.SetSsid(&HSTRING::from(DEFAULT_HOTSPOT_SSID));
            let _ = cfg.SetPassphrase(&HSTRING::from(DEFAULT_HOTSPOT_PASS));
            match tm.ConfigureAccessPointAsync(&cfg) {
                Ok(op) => match op.join() {
                    Ok(()) => {
                        if let Ok(mut g) = snapshot.write() {
                            g.ssid = DEFAULT_HOTSPOT_SSID.to_string();
                            g.passphrase = DEFAULT_HOTSPOT_PASS.to_string();
                            g.status_text = format!(
                                "HOTSPOT CREDENTIALS SET: {} / {}",
                                DEFAULT_HOTSPOT_SSID, DEFAULT_HOTSPOT_PASS
                            );
                            g.last_error = None;
                        }
                        return;
                    }
                    Err(e) => {
                        if let Ok(mut g) = snapshot.write() {
                            g.last_error = Some(format!("Set credentials error: {e}"));
                        }
                    }
                },
                Err(e) => {
                    if let Ok(mut g) = snapshot.write() {
                        g.last_error = Some(format!("ConfigureAccessPointAsync error: {e}"));
                    }
                }
            }
        }

        if let Some(hold) = wfd.as_mut() {
            hold.ssid = DEFAULT_HOTSPOT_SSID.to_string();
            hold.passphrase = DEFAULT_HOTSPOT_PASS.to_string();
        }
        if let Ok(mut g) = snapshot.write() {
            g.ssid = DEFAULT_HOTSPOT_SSID.to_string();
            g.passphrase = DEFAULT_HOTSPOT_PASS.to_string();
        }
    }

    fn start_wifi_direct_fallback(wfd: &mut Option<WfdHold>) -> Result<(String, String), String> {
        if let Some(old) = wfd.take() {
            let _ = old.publisher.Stop();
        }

        let publisher = WiFiDirectAdvertisementPublisher::new()
            .map_err(|e| format!("Wi-Fi Direct publisher init failed: {e}"))?;
        let adv = publisher
            .Advertisement()
            .map_err(|e| format!("Wi-Fi Direct advertisement failed: {e}"))?;
        adv.SetIsAutonomousGroupOwnerEnabled(true)
            .map_err(|e| format!("SetIsAutonomousGroupOwnerEnabled failed: {e}"))?;
        adv.SetListenStateDiscoverability(
            WiFiDirectAdvertisementListenStateDiscoverability::Normal,
        )
        .map_err(|e| format!("SetListenStateDiscoverability failed: {e}"))?;

        let legacy = adv
            .LegacySettings()
            .map_err(|e| format!("LegacySettings failed: {e}"))?;
        legacy
            .SetIsEnabled(true)
            .map_err(|e| format!("LegacySettings.SetIsEnabled failed: {e}"))?;
        legacy
            .SetSsid(&HSTRING::from(DEFAULT_HOTSPOT_SSID))
            .map_err(|e| format!("LegacySettings.SetSsid failed: {e}"))?;

        let cred = PasswordCredential::new()
            .map_err(|e| format!("PasswordCredential init failed: {e}"))?;
        cred.SetPassword(&HSTRING::from(DEFAULT_HOTSPOT_PASS))
            .map_err(|e| format!("SetPassword failed: {e}"))?;
        legacy
            .SetPassphrase(&cred)
            .map_err(|e| format!("LegacySettings.SetPassphrase failed: {e}"))?;

        let listener = WiFiDirectConnectionListener::new().ok();
        publisher
            .Start()
            .map_err(|e| format!("Wi-Fi Direct AP Start failed: {e}"))?;

        let ssid = DEFAULT_HOTSPOT_SSID.to_string();
        let pass = DEFAULT_HOTSPOT_PASS.to_string();
        *wfd = Some(WfdHold {
            publisher,
            _listener: listener,
            ssid: ssid.clone(),
            passphrase: pass.clone(),
        });
        Ok((ssid, pass))
    }

    fn start_hotspot(snapshot: &Arc<RwLock<HotspotSnapshot>>, wfd: &mut Option<WfdHold>) {
        let mobile_err: Option<String>;

        match create_tethering_manager() {
            Ok(tm) => {
                // Ensure non-empty credentials before starting
                if let Ok(cfg) = tm.GetCurrentAccessPointConfiguration() {
                    let cur_ssid = cfg.Ssid().map(|s| s.to_string()).unwrap_or_default();
                    let cur_pass = cfg.Passphrase().map(|s| s.to_string()).unwrap_or_default();
                    if cur_ssid.trim().is_empty() || cur_pass.len() < 8 {
                        let _ = cfg.SetSsid(&HSTRING::from(DEFAULT_HOTSPOT_SSID));
                        let _ = cfg.SetPassphrase(&HSTRING::from(DEFAULT_HOTSPOT_PASS));
                        if let Ok(op) = tm.ConfigureAccessPointAsync(&cfg) {
                            let _ = op.join();
                        }
                    }
                }

                let state = tm
                    .TetheringOperationalState()
                    .unwrap_or(TetheringOperationalState::Off);
                if state == TetheringOperationalState::On {
                    return;
                }

                match tm.StartTetheringAsync() {
                    Ok(op) => match op.join() {
                        Ok(res) => {
                            let status = res.Status().unwrap_or(TetheringOperationStatus::Unknown);
                            if status == TetheringOperationStatus::Success {
                                if let Ok(mut g) = snapshot.write() {
                                    g.last_error = None;
                                    g.transitioning = false;
                                }
                                return;
                            }
                            let extra = res
                                .AdditionalErrorMessage()
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            mobile_err = Some(format!("Windows Hotspot status {status:?} {extra}"));
                        }
                        Err(e) => {
                            mobile_err = Some(format!("StartTetheringAsync failed: {e}"));
                        }
                    },
                    Err(e) => {
                        mobile_err = Some(format!("StartTetheringAsync launch error: {e}"));
                    }
                }
            }
            Err(e) => {
                mobile_err = Some(e);
            }
        }

        // Offline / restricted fallback: Wi-Fi Direct Legacy SoftAP
        match start_wifi_direct_fallback(wfd) {
            Ok((ssid, _)) => {
                if let Ok(mut g) = snapshot.write() {
                    g.transitioning = false;
                    g.status_text = format!("OFFLINE SOFTAP ACTIVE: {ssid}");
                    g.last_error = mobile_err;
                }
            }
            Err(wfd_err) => {
                if let Ok(mut g) = snapshot.write() {
                    g.transitioning = false;
                    g.mode = HotspotMode::Off;
                    g.active = false;
                    g.status_text = "HOTSPOT START FAILED".to_string();
                    g.last_error = Some(match mobile_err {
                        Some(m) => format!("{m} | {wfd_err}"),
                        None => wfd_err,
                    });
                }
            }
        }
    }

    fn stop_hotspot(snapshot: &Arc<RwLock<HotspotSnapshot>>, wfd: &mut Option<WfdHold>) {
        if let Some(hold) = wfd.take() {
            let _ = hold.publisher.Stop();
        }
        if let Ok(tm) = create_tethering_manager()
            && tm.TetheringOperationalState().ok() == Some(TetheringOperationalState::On)
            && let Ok(op) = tm.StopTetheringAsync()
        {
            let _ = op.join();
        }
        if let Ok(mut g) = snapshot.write() {
            g.transitioning = false;
            g.mode = HotspotMode::Off;
            g.active = false;
            g.status_text = "HOTSPOT STOPPED".to_string();
        }
    }

    fn refresh_state(snapshot: &Arc<RwLock<HotspotSnapshot>>, wfd: &Option<WfdHold>) {
        let fw_ok = check_firewall_rule_exists();
        let all_ips = crate::net::detect_local_ipv4s();
        let hotspot_ip = all_ips
            .iter()
            .find(|ip| ip.starts_with("192.168.137."))
            .cloned()
            .or_else(|| {
                if wfd.is_some() {
                    all_ips
                        .iter()
                        .find(|ip| ip.starts_with("192.168.173.") || ip.starts_with("169.254."))
                        .cloned()
                } else {
                    None
                }
            });

        if let Ok(tm) = create_tethering_manager() {
            let op_state = tm
                .TetheringOperationalState()
                .unwrap_or(TetheringOperationalState::Off);
            let clients = tm.ClientCount().unwrap_or(0);
            let max_clients = tm.MaxClientCount().unwrap_or(8);
            let (ssid, pass) = tm
                .GetCurrentAccessPointConfiguration()
                .ok()
                .map(|c| {
                    (
                        c.Ssid().map(|s| s.to_string()).unwrap_or_default(),
                        c.Passphrase().map(|s| s.to_string()).unwrap_or_default(),
                    )
                })
                .unwrap_or_else(|| {
                    (
                        DEFAULT_HOTSPOT_SSID.to_string(),
                        DEFAULT_HOTSPOT_PASS.to_string(),
                    )
                });

            if op_state == TetheringOperationalState::On {
                if let Ok(mut g) = snapshot.write() {
                    g.mode = HotspotMode::MobileHotspot;
                    g.active = true;
                    g.transitioning = false;
                    g.ssid = if ssid.is_empty() {
                        DEFAULT_HOTSPOT_SSID.to_string()
                    } else {
                        ssid.clone()
                    };
                    g.passphrase = pass;
                    g.gateway_ip = hotspot_ip.or_else(|| Some("192.168.137.1".to_string()));
                    g.client_count = clients;
                    g.max_clients = max_clients;
                    g.firewall_rule_ok = fw_ok;
                    g.status_text =
                        format!("ACTIVE: {} | {} / {} DEVICES", g.ssid, clients, max_clients);
                }
                return;
            } else if op_state == TetheringOperationalState::InTransition {
                if let Ok(mut g) = snapshot.write() {
                    g.mode = HotspotMode::Transitioning;
                    g.transitioning = true;
                    g.firewall_rule_ok = fw_ok;
                    g.status_text = "HOTSPOT TRANSITIONING...".to_string();
                }
                return;
            }

            if wfd.is_none() {
                if let Ok(mut g) = snapshot.write() {
                    g.mode = HotspotMode::Off;
                    g.active = false;
                    g.transitioning = false;
                    g.ssid = if ssid.is_empty() {
                        DEFAULT_HOTSPOT_SSID.to_string()
                    } else {
                        ssid
                    };
                    g.passphrase = pass;
                    g.gateway_ip = hotspot_ip;
                    g.client_count = 0;
                    g.max_clients = max_clients;
                    g.firewall_rule_ok = fw_ok;
                    if g.last_error.is_none() {
                        g.status_text = "HOTSPOT READY (OFF)".to_string();
                    }
                }
                return;
            }
        }

        if let Some(hold) = wfd {
            let started = hold.publisher.Status().ok()
                == Some(WiFiDirectAdvertisementPublisherStatus::Started);
            if let Ok(mut g) = snapshot.write() {
                g.mode = if started {
                    HotspotMode::WifiDirectAp
                } else {
                    HotspotMode::Off
                };
                g.active = started;
                g.transitioning = false;
                g.ssid = hold.ssid.clone();
                g.passphrase = hold.passphrase.clone();
                g.gateway_ip = hotspot_ip;
                g.client_count = 0;
                g.max_clients = 8;
                g.firewall_rule_ok = fw_ok;
                g.status_text = if started {
                    format!("OFFLINE SOFTAP: {} (NO INTERNET)", hold.ssid)
                } else {
                    "OFFLINE SOFTAP STOPPED".to_string()
                };
            }
        } else if let Ok(mut g) = snapshot.write() {
            g.mode = HotspotMode::Off;
            g.active = false;
            g.transitioning = false;
            g.gateway_ip = hotspot_ip;
            g.firewall_rule_ok = fw_ok;
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::*;

    pub(super) fn worker_loop(
        rx: mpsc::Receiver<HotspotCmd>,
        snapshot: Arc<RwLock<HotspotSnapshot>>,
    ) {
        if let Ok(mut g) = snapshot.write() {
            g.status_text = "HOTSPOT CONTROLLER WINDOWS-ONLY".to_string();
            g.firewall_rule_ok = true;
        }
        while let Ok(cmd) = rx.recv() {
            if matches!(cmd, HotspotCmd::Shutdown) {
                break;
            }
        }
    }
}
