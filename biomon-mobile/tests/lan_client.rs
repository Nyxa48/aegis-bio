use biomon_mobile::net::{
    BiomonLanClient, BiomonLanConfig, ConnectionState, DISCOVERY_MAGIC_REQ, DISCOVERY_MAGIC_RESP,
    DiscoveryBeacon, DiscoveryProbe, HubDownlinkMessage, LinkEvent, OutboundFrame,
};
use serde_json::json;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, UdpSocket};

struct MockHub {
    discovery_port: u16,
    tcp_port: u16,
    seen_crews: Arc<Mutex<Vec<String>>>,
    received_types: Arc<Mutex<Vec<String>>>,
    _rt: tokio::runtime::Runtime,
}

impl MockHub {
    fn start(send_phase_sync_on_batch: bool) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("mock hub tokio rt");

        let std_udp = std::net::UdpSocket::bind("127.0.0.1:0").expect("bind mock udp");
        std_udp.set_nonblocking(true).expect("udp nonblocking");
        let discovery_port = std_udp.local_addr().unwrap().port();

        let std_tcp = std::net::TcpListener::bind("127.0.0.1:0").expect("bind mock tcp");
        std_tcp.set_nonblocking(true).expect("tcp nonblocking");
        let tcp_port = std_tcp.local_addr().unwrap().port();

        let seen_crews = Arc::new(Mutex::new(Vec::new()));
        let received_types = Arc::new(Mutex::new(Vec::new()));
        let crews_clone = Arc::clone(&seen_crews);
        let types_clone = Arc::clone(&received_types);

        rt.spawn(async move {
            let udp = UdpSocket::from_std(std_udp).expect("tokio udp");
            tokio::spawn(async move {
                let mut buf = [0u8; 2048];
                while let Ok((n, peer)) = udp.recv_from(&mut buf).await {
                    if let Ok(probe) = serde_json::from_slice::<DiscoveryProbe>(&buf[..n])
                        && probe.kind == DISCOVERY_MAGIC_REQ
                    {
                        let beacon = DiscoveryBeacon {
                            kind: DISCOVERY_MAGIC_RESP.to_string(),
                            proto: 1,
                            hub_id: "MOCK-HUB".to_string(),
                            tcp_port,
                            mission_phase: "ORBIT".to_string(),
                        };
                        if let Ok(bytes) = serde_json::to_vec(&beacon) {
                            let _ = udp.send_to(&bytes, peer).await;
                        }
                    }
                }
            });

            let tcp = TcpListener::from_std(std_tcp).expect("tokio tcp");
            while let Ok((stream, _)) = tcp.accept().await {
                let crews_inner = Arc::clone(&crews_clone);
                let types_inner = Arc::clone(&types_clone);
                tokio::spawn(async move {
                    let (read_half, mut write_half) = tokio::io::split(stream);
                    let mut lines = BufReader::new(read_half).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        let Ok(val) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
                            continue;
                        };
                        let crew_id = val["crew_id"].as_str().unwrap_or_default().to_string();
                        let pkt_type = val["packet_type"].as_str().unwrap_or_default().to_string();

                        if !crew_id.is_empty()
                            && let Ok(mut g) = crews_inner.lock()
                        {
                            g.push(crew_id.clone());
                        }
                        if let Ok(mut g) = types_inner.lock() {
                            g.push(pkt_type.clone());
                        }

                        if pkt_type == "HELLO" {
                            let ack = HubDownlinkMessage::HelloAck {
                                hub_id: "MOCK-HUB".to_string(),
                                mission_phase: "EVA".to_string(),
                                distance_au: 1.4,
                                one_way_light_min: 11.6,
                                assigned_crew_id: crew_id.clone(),
                                assigned_callsign: val["payload"]["callsign"]
                                    .as_str()
                                    .unwrap_or("ARES-1")
                                    .to_string(),
                            };
                            let s = serde_json::to_string(&ack).unwrap() + "\n";
                            if write_half.write_all(s.as_bytes()).await.is_err() {
                                break;
                            }
                            let _ = write_half.flush().await;
                        } else if pkt_type == "TELEMETRY_BATCH_SYNC" {
                            let batch_id = val["payload"]["batch_id"].as_u64().unwrap_or(1);
                            let ack = HubDownlinkMessage::SyncAck {
                                batch_id,
                                synced_records: 2,
                            };
                            let s = serde_json::to_string(&ack).unwrap() + "\n";
                            let _ = write_half.write_all(s.as_bytes()).await;
                            if send_phase_sync_on_batch {
                                let ps = HubDownlinkMessage::PhaseSync {
                                    phase: "SURFACE".to_string(),
                                    apply_to_all: true,
                                    target_crew_id: None,
                                };
                                let ps_line = serde_json::to_string(&ps).unwrap() + "\n";
                                let _ = write_half.write_all(ps_line.as_bytes()).await;
                            }
                            let _ = write_half.flush().await;
                        }
                    }
                });
            }
        });

        Self {
            discovery_port,
            tcp_port,
            seen_crews,
            received_types,
            _rt: rt,
        }
    }
}

#[test]
fn auto_discovery_handshake_sync_and_blackout_recovery() {
    let hub = MockHub::start(true);
    let mut client = BiomonLanClient::with_config(BiomonLanConfig {
        hub_override_addr: None,
        discovery_port: hub.discovery_port,
        tcp_fallback_port: hub.tcp_port,
        crew_id: "AST-01".to_string(),
        callsign: "ARES-1".to_string(),
        active_phase: "ORBIT".to_string(),
    });

    // Wait for auto-discovery + HelloAck -> Online
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut got_hello_ack = false;
    while Instant::now() < deadline {
        while let Some(ev) = client.try_recv_event() {
            if let LinkEvent::Downlink(HubDownlinkMessage::HelloAck { mission_phase, .. }) = ev {
                assert_eq!(mission_phase, "EVA");
                got_hello_ack = true;
            }
        }
        if client.connection_state() == ConnectionState::Online && got_hello_ack {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert_eq!(client.connection_state(), ConnectionState::Online);
    assert!(
        got_hello_ack,
        "client should receive HelloAck from discovered Hub"
    );
    assert!(client.target_hub_addr().contains(&hub.tcp_port.to_string()));

    // Queue a batch sync frame -> SyncAck + PhaseSync
    let batch_id = client.next_batch_id();
    let queued = client.queue_frame(OutboundFrame {
        crew_id: "AST-01".to_string(),
        packet_type: "TELEMETRY_BATCH_SYNC".to_string(),
        payload: json!({ "batch_id": batch_id, "hr_bpm": 84.0 }),
    });
    assert!(queued);

    let mut got_sync_ack = false;
    let mut got_phase_sync = false;
    let sync_deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < sync_deadline && (!got_sync_ack || !got_phase_sync) {
        while let Some(ev) = client.try_recv_event() {
            match ev {
                LinkEvent::Downlink(HubDownlinkMessage::SyncAck {
                    batch_id: ack_id,
                    synced_records,
                }) => {
                    assert_eq!(ack_id, batch_id);
                    assert_eq!(synced_records, 2);
                    got_sync_ack = true;
                }
                LinkEvent::Downlink(HubDownlinkMessage::PhaseSync { phase, .. }) => {
                    assert_eq!(phase, "SURFACE");
                    got_phase_sync = true;
                }
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(got_sync_ack, "expected SyncAck");
    assert!(got_phase_sync, "expected PhaseSync");
    assert!(client.frames_sent() >= 1);
    {
        let rtypes = hub.received_types.lock().unwrap();
        assert!(rtypes.contains(&"HELLO".to_string()));
        assert!(rtypes.contains(&"TELEMETRY_BATCH_SYNC".to_string()));
    }

    // Toggle manual blackout -> Offline & rejects queue_frame (DB preserves records offline)
    let state_after_blackout = client.toggle_connection_state();
    assert_eq!(state_after_blackout, ConnectionState::Offline);
    assert!(client.is_manual_blackout());
    assert!(!client.queue_frame(OutboundFrame {
        crew_id: "AST-01".to_string(),
        packet_type: "TELEMETRY_LIVE".to_string(),
        payload: json!({ "hr_bpm": 70.0 }),
    }));

    // Wait briefly for worker loop to emit Disconnected
    let off_deadline = Instant::now() + Duration::from_secs(2);
    let mut got_disconnected = false;
    while Instant::now() < off_deadline && !got_disconnected {
        while let Some(ev) = client.try_recv_event() {
            if matches!(ev, LinkEvent::Disconnected) {
                got_disconnected = true;
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(got_disconnected, "expected Disconnected event on blackout");

    // Disable blackout -> automatically rediscovers and reconnects
    client.toggle_connection_state();
    assert!(!client.is_manual_blackout());
    let rec_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < rec_deadline && client.connection_state() != ConnectionState::Online {
        std::thread::sleep(Duration::from_millis(25));
    }
    assert_eq!(client.connection_state(), ConnectionState::Online);
}

#[test]
fn four_simultaneous_crew_clients_auto_discover_without_conflict() {
    let hub = MockHub::start(false);
    let mut clients: Vec<BiomonLanClient> = [
        ("AST-01", "ARES-1"),
        ("AST-02", "ARES-2"),
        ("AST-03", "ARES-3"),
        ("AST-04", "ARES-4"),
    ]
    .into_iter()
    .map(|(crew_id, callsign)| {
        BiomonLanClient::with_config(BiomonLanConfig {
            hub_override_addr: None,
            discovery_port: hub.discovery_port,
            tcp_fallback_port: hub.tcp_port,
            crew_id: crew_id.to_string(),
            callsign: callsign.to_string(),
            active_phase: "ORBIT".to_string(),
        })
    })
    .collect();

    let deadline = Instant::now() + Duration::from_secs(6);
    while Instant::now() < deadline {
        if clients
            .iter()
            .all(|c| c.connection_state() == ConnectionState::Online)
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(30));
    }

    for (idx, client) in clients.iter_mut().enumerate() {
        assert_eq!(
            client.connection_state(),
            ConnectionState::Online,
            "client {} failed to come online",
            idx + 1
        );
    }

    let seen: HashSet<String> = hub.seen_crews.lock().unwrap().iter().cloned().collect();
    for expected in ["AST-01", "AST-02", "AST-03", "AST-04"] {
        assert!(seen.contains(expected), "missing HELLO for {expected}");
    }
}


#[test]
fn client_adopts_hub_assigned_slot_when_default_is_occupied() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("rt");

    let std_udp = std::net::UdpSocket::bind("127.0.0.1:0").expect("bind udp");
    std_udp.set_nonblocking(true).unwrap();
    let disc_port = std_udp.local_addr().unwrap().port();

    let std_tcp = std::net::TcpListener::bind("127.0.0.1:0").expect("bind tcp");
    std_tcp.set_nonblocking(true).unwrap();
    let tcp_port = std_tcp.local_addr().unwrap().port();

    let seen_after_hello = Arc::new(Mutex::new(Vec::<(String, String)>::new()));
    let seen_clone = Arc::clone(&seen_after_hello);

    rt.spawn(async move {
        let udp = UdpSocket::from_std(std_udp).unwrap();
        tokio::spawn(async move {
            let mut buf = [0u8; 2048];
            while let Ok((n, peer)) = udp.recv_from(&mut buf).await {
                if let Ok(probe) = serde_json::from_slice::<DiscoveryProbe>(&buf[..n])
                    && probe.kind == DISCOVERY_MAGIC_REQ
                {
                    let beacon = DiscoveryBeacon {
                        kind: DISCOVERY_MAGIC_RESP.to_string(),
                        proto: 1,
                        hub_id: "MOCK-HUB".to_string(),
                        tcp_port,
                        mission_phase: "ORBIT".to_string(),
                    };
                    let _ = udp.send_to(&serde_json::to_vec(&beacon).unwrap(), peer).await;
                }
            }
        });

        let tcp = TcpListener::from_std(std_tcp).unwrap();
        while let Ok((stream, _)) = tcp.accept().await {
            let sc = Arc::clone(&seen_clone);
            tokio::spawn(async move {
                let (read_half, mut write_half) = tokio::io::split(stream);
                let mut lines = BufReader::new(read_half).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let Ok(val) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
                        continue;
                    };
                    let cid = val["crew_id"].as_str().unwrap_or_default().to_string();
                    let ptype = val["packet_type"].as_str().unwrap_or_default().to_string();
                    if let Ok(mut g) = sc.lock() {
                        g.push((ptype.clone(), cid));
                    }
                    if ptype == "HELLO" {
                        let ack = HubDownlinkMessage::HelloAck {
                            hub_id: "MOCK-HUB".to_string(),
                            mission_phase: "ORBIT".to_string(),
                            distance_au: 1.32,
                            one_way_light_min: 11.0,
                            assigned_crew_id: "AST-02".to_string(),
                            assigned_callsign: "ARES-2".to_string(),
                        };
                        let s = serde_json::to_string(&ack).unwrap() + "\n";
                        let _ = write_half.write_all(s.as_bytes()).await;
                        let _ = write_half.flush().await;
                    }
                }
            });
        }
    });

    let mut client = BiomonLanClient::with_config(BiomonLanConfig {
        hub_override_addr: None,
        discovery_port: disc_port,
        tcp_fallback_port: tcp_port,
        crew_id: "AST-01".to_string(),
        callsign: "ARES-1".to_string(),
        active_phase: "ORBIT".to_string(),
    });

    let deadline = Instant::now() + Duration::from_secs(4);
    let mut got_assigned = false;
    while Instant::now() < deadline && !got_assigned {
        while let Some(ev) = client.try_recv_event() {
            if let LinkEvent::Downlink(HubDownlinkMessage::HelloAck {
                assigned_crew_id,
                assigned_callsign,
                ..
            }) = ev
            {
                assert_eq!(assigned_crew_id, "AST-02");
                assert_eq!(assigned_callsign, "ARES-2");
                got_assigned = true;
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(got_assigned);
    assert_eq!(client.crew_id(), "AST-02");
}
