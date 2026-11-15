use hub_desktop::net::{
    DISCOVERY_MAGIC_REQ, DISCOVERY_MAGIC_RESP, DiscoveryBeacon, DiscoveryProbe, HubDownlinkMessage,
    HubLanServer, HubNetEvent,
};
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpStream, UdpSocket};

#[test]
fn lan_server_discovery_session_sync_and_disconnect() {
    let mut server = HubLanServer::start_with_ports("127.0.0.1", 0, 0);
    assert!(server.is_ready(), "TCP server should bind on :0");
    assert!(server.discovery_ready(), "UDP server should bind on :0");
    assert!(server.last_error().is_none());
    assert!(!server.local_ipv4s().is_empty());

    let tcp_port = server.tcp_port();
    let disc_port = server.discovery_port();
    assert_ne!(tcp_port, 0);
    assert_ne!(disc_port, 0);

    server.update_hub_metadata("EVA", 1.4501, 12.06);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    rt.block_on(async {
        // 1. UDP Discovery Probe -> DiscoveryBeacon
        let udp = UdpSocket::bind("127.0.0.1:0").await.expect("bind udp");
        let probe = DiscoveryProbe {
            kind: DISCOVERY_MAGIC_REQ.to_string(),
            proto: 1,
            crew_id: "AST-02".to_string(),
            callsign: "ARES-2".to_string(),
        };
        let bytes = serde_json::to_vec(&probe).expect("serialize probe");
        udp.send_to(&bytes, format!("127.0.0.1:{disc_port}"))
            .await
            .expect("send udp probe");

        let mut buf = [0u8; 2048];
        let (n, _) = tokio::time::timeout(Duration::from_secs(2), udp.recv_from(&mut buf))
            .await
            .expect("udp beacon timeout")
            .expect("udp recv");
        let beacon: DiscoveryBeacon =
            serde_json::from_slice(&buf[..n]).expect("parse discovery beacon");
        assert_eq!(beacon.kind, DISCOVERY_MAGIC_RESP);
        assert_eq!(beacon.tcp_port, tcp_port);
        assert_eq!(beacon.mission_phase, "EVA");

        // 2. Connect TCP & send HELLO -> HelloAck
        let stream = TcpStream::connect(format!("127.0.0.1:{tcp_port}"))
            .await
            .expect("tcp connect");
        let (read_half, mut write_half) = stream.into_split();
        let mut lines = BufReader::new(read_half).lines();

        let hello = json!({
            "crew_id": "AST-02",
            "packet_type": "HELLO",
            "payload": { "callsign": "ARES-2", "active_phase": "EVA" }
        });
        write_half
            .write_all(format!("{hello}\n").as_bytes())
            .await
            .expect("write HELLO");
        write_half.flush().await.expect("flush HELLO");

        let ack_line = tokio::time::timeout(Duration::from_secs(2), lines.next_line())
            .await
            .expect("HelloAck timeout")
            .expect("HelloAck io")
            .expect("HelloAck EOF");
        let ack: HubDownlinkMessage =
            serde_json::from_str(ack_line.trim()).expect("parse HelloAck");
        match ack {
            HubDownlinkMessage::HelloAck {
                mission_phase,
                distance_au,
                ..
            } => {
                assert_eq!(mission_phase, "EVA");
                assert!((distance_au - 1.4501).abs() < 1e-6);
            }
            other => panic!("expected HelloAck, got {other:?}"),
        }

        // 3. Send CREW_INPUT & TELEMETRY_BATCH_SYNC
        let input_pkt = json!({
            "crew_id": "AST-02",
            "packet_type": "CREW_INPUT",
            "payload": { "batch_id": 7, "water_ml": 500.0 }
        });
        write_half
            .write_all(format!("{input_pkt}\n").as_bytes())
            .await
            .expect("write CREW_INPUT");

        let batch_pkt = json!({
            "crew_id": "AST-02",
            "packet_type": "TELEMETRY_BATCH_SYNC",
            "payload": { "batch_id": 8, "hr_bpm": 88.0 }
        });
        write_half
            .write_all(format!("{batch_pkt}\n").as_bytes())
            .await
            .expect("write TELEMETRY_BATCH_SYNC");
        write_half.flush().await.expect("flush packets");

        // Poll Hub events until ClientConnected + both Packets arrive
        let mut got_connected = false;
        let mut packet_types = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline && (!got_connected || packet_types.len() < 3) {
            while let Some(ev) = server.try_recv_event() {
                match ev {
                    HubNetEvent::ClientConnected {
                        crew_id, callsign, ..
                    } => {
                        assert_eq!(crew_id, "AST-02");
                        assert_eq!(callsign, "ARES-2");
                        got_connected = true;
                    }
                    HubNetEvent::Packet { packet, .. } => {
                        packet_types.push(packet.packet_type);
                    }
                    _ => {}
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(got_connected, "expected ClientConnected event");
        assert!(packet_types.contains(&"HELLO".to_string()));
        assert!(packet_types.contains(&"CREW_INPUT".to_string()));
        assert!(packet_types.contains(&"TELEMETRY_BATCH_SYNC".to_string()));

        assert!(server.crew_has_active_session("AST-02"));
        assert!(server.online_crew_peers().contains_key("AST-02"));
        assert_eq!(server.connected_clients(), 1);

        // 4. Downlink SyncAck + PhaseSync broadcast
        let sent_ack = server.send_to_crew(
            "AST-02",
            HubDownlinkMessage::SyncAck {
                batch_id: 8,
                synced_records: 3,
            },
        );
        assert_eq!(sent_ack, 1);

        let sent_phase = server.broadcast_downlink(HubDownlinkMessage::PhaseSync {
            phase: "TRANSIT".to_string(),
            apply_to_all: true,
            target_crew_id: None,
        });
        assert_eq!(sent_phase, 1);

        let line1 = tokio::time::timeout(Duration::from_secs(2), lines.next_line())
            .await
            .expect("SyncAck timeout")
            .expect("SyncAck io")
            .expect("SyncAck EOF");
        let msg1: HubDownlinkMessage = serde_json::from_str(line1.trim()).expect("parse SyncAck");
        assert!(matches!(
            msg1,
            HubDownlinkMessage::SyncAck {
                batch_id: 8,
                synced_records: 3
            }
        ));

        let line2 = tokio::time::timeout(Duration::from_secs(2), lines.next_line())
            .await
            .expect("PhaseSync timeout")
            .expect("PhaseSync io")
            .expect("PhaseSync EOF");
        let msg2: HubDownlinkMessage = serde_json::from_str(line2.trim()).expect("parse PhaseSync");
        assert!(matches!(
            msg2,
            HubDownlinkMessage::PhaseSync {
                ref phase,
                apply_to_all: true,
                ..
            } if phase == "TRANSIT"
        ));

        // 5. Disconnect cleanup
        drop(write_half);
        drop(lines);

        let mut got_disconnected = false;
        let disc_deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < disc_deadline && !got_disconnected {
            while let Some(ev) = server.try_recv_event() {
                if let HubNetEvent::ClientDisconnected { crew_id, .. } = ev
                    && crew_id == "AST-02"
                {
                    got_disconnected = true;
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(got_disconnected, "expected ClientDisconnected event");
        assert_eq!(server.connected_clients(), 0);
        assert!(!server.crew_has_active_session("AST-02"));
    });
}

#[test]
fn six_slot_auto_increment_eviction_and_seventh_client_rejection() {
    let mut server = HubLanServer::start_with_ports("127.0.0.1", 0, 0);
    assert!(server.is_ready());
    let tcp_port = server.tcp_port();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    rt.block_on(async {
        let mut conns = Vec::new();
        let mut assigned_slots = Vec::new();

        // 1. Connect 6 devices that ALL default-request "AST-01"
        for i in 1..=6_u32 {
            let stream = TcpStream::connect(format!("127.0.0.1:{tcp_port}"))
                .await
                .expect("tcp connect");
            let (read_half, mut write_half) = stream.into_split();
            let mut lines = BufReader::new(read_half).lines();

            let hello = json!({
                "crew_id": "AST-01",
                "packet_type": "HELLO",
                "payload": {
                    "callsign": "ARES-1",
                    "active_phase": "ORBIT",
                    "device_id": format!("tablet-{i}")
                }
            });
            write_half
                .write_all(format!("{hello}\n").as_bytes())
                .await
                .expect("write HELLO");
            write_half.flush().await.expect("flush HELLO");

            let ack_line = tokio::time::timeout(Duration::from_secs(2), lines.next_line())
                .await
                .expect("HelloAck timeout")
                .expect("HelloAck io")
                .expect("HelloAck EOF");
            let ack: HubDownlinkMessage =
                serde_json::from_str(ack_line.trim()).expect("parse HelloAck");
            match ack {
                HubDownlinkMessage::HelloAck {
                    assigned_crew_id, ..
                } => {
                    assigned_slots.push(assigned_crew_id);
                }
                other => panic!("expected HelloAck for device {i}, got {other:?}"),
            }
            conns.push((lines, write_half));
        }

        assert_eq!(
            assigned_slots,
            vec!["AST-01", "AST-02", "AST-03", "AST-04", "AST-05", "AST-06"]
        );
        assert_eq!(server.online_crew_peers().len(), 6);

        // Drain initial events, then have tablet #2 send an un-updated AST-01 frame;
        // verify Hub pins it strictly to AST-02.
        while server.try_recv_event().is_some() {}
        let stale_pkt = json!({
            "crew_id": "AST-01",
            "packet_type": "TELEMETRY_LIVE",
            "payload": { "hr_bpm": 91.0 }
        });
        conns[1]
            .1
            .write_all(format!("{stale_pkt}\n").as_bytes())
            .await
            .expect("write telemetry");
        conns[1].1.flush().await.expect("flush telemetry");

        let mut pinned_ok = false;
        let dl = Instant::now() + Duration::from_secs(2);
        while Instant::now() < dl && !pinned_ok {
            while let Some(ev) = server.try_recv_event() {
                if let HubNetEvent::Packet { packet, .. } = ev
                    && packet.packet_type == "TELEMETRY_LIVE"
                {
                    assert_eq!(packet.crew_id, "AST-02");
                    pinned_ok = true;
                }
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
        assert!(pinned_ok, "expected telemetry from conn 1 pinned to AST-02");

        // 2. 7th device attempting to connect must get SlotFull { max_slots: 6 }
        let stream7 = TcpStream::connect(format!("127.0.0.1:{tcp_port}"))
            .await
            .expect("tcp connect 7");
        let (r7, mut w7) = stream7.into_split();
        let mut l7 = BufReader::new(r7).lines();
        let hello7 = json!({
            "crew_id": "AST-01",
            "packet_type": "HELLO",
            "payload": { "callsign": "ARES-1", "device_id": "tablet-7" }
        });
        w7.write_all(format!("{hello7}\n").as_bytes())
            .await
            .expect("write HELLO 7");
        w7.flush().await.expect("flush HELLO 7");
        let msg7_line = tokio::time::timeout(Duration::from_secs(2), l7.next_line())
            .await
            .expect("SlotFull timeout")
            .expect("SlotFull io")
            .expect("SlotFull EOF");
        let msg7: HubDownlinkMessage = serde_json::from_str(msg7_line.trim()).expect("parse 7");
        assert!(matches!(
            msg7,
            HubDownlinkMessage::SlotFull { max_slots: 6, .. }
        ));
        assert_eq!(server.online_crew_peers().len(), 6);

        // 3. Disconnect AST-02; a new client requesting AST-01 should claim freed AST-02
        let conn2 = conns.remove(1);
        drop(conn2);
        let free_dl = Instant::now() + Duration::from_secs(2);
        while Instant::now() < free_dl && server.crew_has_active_session("AST-02") {
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
        assert!(!server.crew_has_active_session("AST-02"));

        let stream_new = TcpStream::connect(format!("127.0.0.1:{tcp_port}"))
            .await
            .expect("tcp connect new");
        let (rn, mut wn) = stream_new.into_split();
        let mut ln = BufReader::new(rn).lines();
        let hello_new = json!({
            "crew_id": "AST-01",
            "packet_type": "HELLO",
            "payload": { "callsign": "ARES-1", "device_id": "tablet-8" }
        });
        wn.write_all(format!("{hello_new}\n").as_bytes())
            .await
            .expect("write HELLO new");
        wn.flush().await.expect("flush HELLO new");
        let ack_new_line = tokio::time::timeout(Duration::from_secs(2), ln.next_line())
            .await
            .expect("ack new timeout")
            .expect("ack new io")
            .expect("ack new EOF");
        let ack_new: HubDownlinkMessage =
            serde_json::from_str(ack_new_line.trim()).expect("parse ack new");
        assert!(matches!(
            ack_new,
            HubDownlinkMessage::HelloAck {
                ref assigned_crew_id,
                ..
            } if assigned_crew_id == "AST-02"
        ));
    });
}

#[test]
fn same_device_reconnection_evicts_old_socket_cleanly_without_losing_slot() {
    let mut server = HubLanServer::start_with_ports("127.0.0.1", 0, 0);
    assert!(server.is_ready());
    let tcp_port = server.tcp_port();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    rt.block_on(async {
        // 1. Phone 1 connects on Socket A -> assigned AST-01
        let s1_old = TcpStream::connect(format!("127.0.0.1:{tcp_port}"))
            .await
            .expect("connect s1_old");
        let (r1a, mut w1a) = s1_old.into_split();
        let mut l1a = BufReader::new(r1a).lines();
        let hello1 = json!({
            "crew_id": "AST-01",
            "packet_type": "HELLO",
            "payload": { "callsign": "ARES-1", "device_id": "phone-1" }
        });
        w1a.write_all(format!("{hello1}\n").as_bytes())
            .await
            .unwrap();
        w1a.flush().await.unwrap();
        let ack1a: HubDownlinkMessage = serde_json::from_str(
            tokio::time::timeout(Duration::from_secs(2), l1a.next_line())
                .await
                .unwrap()
                .unwrap()
                .unwrap()
                .trim(),
        )
        .unwrap();
        assert!(matches!(
            ack1a,
            HubDownlinkMessage::HelloAck {
                ref assigned_crew_id,
                ..
            } if assigned_crew_id == "AST-01"
        ));

        while server.try_recv_event().is_some() {}

        // 2. Phone 1 reconnects on Socket B BEFORE Socket A closes
        let s1_new = TcpStream::connect(format!("127.0.0.1:{tcp_port}"))
            .await
            .expect("connect s1_new");
        let (r1b, mut w1b) = s1_new.into_split();
        let mut l1b = BufReader::new(r1b).lines();
        w1b.write_all(format!("{hello1}\n").as_bytes())
            .await
            .unwrap();
        w1b.flush().await.unwrap();
        let ack1b: HubDownlinkMessage = serde_json::from_str(
            tokio::time::timeout(Duration::from_secs(2), l1b.next_line())
                .await
                .unwrap()
                .unwrap()
                .unwrap()
                .trim(),
        )
        .unwrap();
        assert!(matches!(
            ack1b,
            HubDownlinkMessage::HelloAck {
                ref assigned_crew_id,
                ..
            } if assigned_crew_id == "AST-01"
        ));

        // Socket A must terminate immediately via watch shutdown without emitting ClientDisconnected
        let old_eof = tokio::time::timeout(Duration::from_secs(2), l1a.next_line())
            .await
            .expect("old socket should close immediately upon eviction");
        assert!(matches!(old_eof, Ok(None) | Err(_)));
        drop(w1a);
        drop(l1a);

        tokio::time::sleep(Duration::from_millis(80)).await;
        while let Some(ev) = server.try_recv_event() {
            assert!(
                !matches!(ev, HubNetEvent::ClientDisconnected { .. }),
                "evicted stale socket must not emit ClientDisconnected: {ev:?}"
            );
        }
        assert!(server.crew_has_active_session("AST-01"));

        // 3. Phone 2 connects requesting AST-01 -> must be bumped to AST-02
        let s2 = TcpStream::connect(format!("127.0.0.1:{tcp_port}"))
            .await
            .expect("connect s2");
        let (r2, mut w2) = s2.into_split();
        let mut l2 = BufReader::new(r2).lines();
        let hello2 = json!({
            "crew_id": "AST-01",
            "packet_type": "HELLO",
            "payload": { "callsign": "ARES-1", "device_id": "phone-2" }
        });
        w2.write_all(format!("{hello2}\n").as_bytes())
            .await
            .unwrap();
        w2.flush().await.unwrap();
        let ack2: HubDownlinkMessage = serde_json::from_str(
            tokio::time::timeout(Duration::from_secs(2), l2.next_line())
                .await
                .unwrap()
                .unwrap()
                .unwrap()
                .trim(),
        )
        .unwrap();
        assert!(matches!(
            ack2,
            HubDownlinkMessage::HelloAck {
                ref assigned_crew_id,
                ..
            } if assigned_crew_id == "AST-02"
        ));
        assert_eq!(server.online_crew_peers().len(), 2);
    });
}
