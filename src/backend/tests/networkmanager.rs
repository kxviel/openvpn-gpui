use super::super::{
    VpnBackend, VpnEvent, VpnRequest, connection::active_uuids, nmcli::run_nmcli,
    worker::handle_request,
};
use crate::profile::{self, Profiles};
use std::{
    fs,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
use uuid::Uuid;

fn wait_for_activation(uuid: Uuid) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if active_uuids().unwrap().contains(&uuid) {
            return;
        }
        assert!(Instant::now() < deadline, "test VPN never began activation");
        thread::sleep(Duration::from_millis(100));
    }
}
#[test]
#[ignore = "Creates and removes two disposable NetworkManager profiles; requires nmcli, its OpenVPN plugin, and openssl"]
fn networkmanager_import_cancel_and_remove() {
    let base_networks = run_nmcli(
        &["-t", "-f", "UUID,TYPE", "connection", "show", "--active"],
        8,
        None,
    )
    .unwrap()
    .lines()
    .filter_map(|line| {
        let (uuid, kind) = line.split_once(':')?;
        matches!(kind, "802-11-wireless" | "802-3-ethernet").then(|| uuid.to_owned())
    })
    .collect::<Vec<_>>();
    let snapshot = |uuid: &str| {
        run_nmcli(
            &[
                "-g",
                "GENERAL.STATE,IP4.ROUTE,IP4.DNS,IP6.ROUTE,IP6.DNS",
                "connection",
                "show",
                "uuid",
                uuid,
            ],
            8,
            None,
        )
        .unwrap()
    };
    let baseline = base_networks
        .iter()
        .map(|uuid| snapshot(uuid))
        .collect::<Vec<_>>();
    struct Cleanup(Vec<Uuid>);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            for id in &self.0 {
                let _ = run_nmcli(&["connection", "delete", "uuid", &id.to_string()], 10, None);
            }
        }
    }
    let mut cleanup = Cleanup(Vec::new());
    // NetworkManager has a private /tmp namespace. Use the same filesystem
    // location as real profiles so the VPN service can read the fixture.
    let root = tempfile::Builder::new()
        .prefix("openvpn-gpui-test-")
        .tempdir_in(dirs::data_local_dir().unwrap())
        .unwrap();
    let ca = root.path().join("fixture-ca.pem");
    let key = root.path().join("fixture-key.pem");
    assert!(
        Command::new("openssl")
            .args(["req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout"])
            .arg(&key)
            .args(["-out"])
            .arg(&ca)
            .args(["-days", "1", "-subj", "/CN=Local integration fixture"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success()
    );
    let source = root.path().join("fixture.ovpn");
    // Loopback discard port: this test cannot connect to a real VPN or install routes.
    fs::write(&source, "client\ndev tun\nproto udp\nremote 127.0.0.1 9\nca fixture-ca.pem\nauth-user-pass\nremote-cert-tls server\nconnect-retry-max 1\n").unwrap();
    let data = root.path().join("data");
    let mut store = Profiles::default();
    let (tx, _rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    for name in ["Integration fixture one", "Integration fixture two"] {
        handle_request(
            VpnRequest::Import {
                path: source.clone(),
                name: name.into(),
                credentials: None,
            },
            &data,
            &mut store,
            &tx,
            &cancel,
        )
        .unwrap();
        cleanup.0.push(store.selected().unwrap().nm_uuid);
    }
    assert_eq!(profile::load(&data).unwrap().profiles.len(), 2);
    assert_ne!(store.profiles[0].nm_uuid, store.profiles[1].nm_uuid);
    let first = store.profiles[0].id;
    handle_request(VpnRequest::Select(first), &data, &mut store, &tx, &cancel).unwrap();
    assert_eq!(profile::load(&data).unwrap().selected, Some(first));
    handle_request(
        VpnRequest::SaveCredentials {
            id: first,
            username: "integration-user".into(),
            password: Some("integration-password".into()),
        },
        &data,
        &mut store,
        &tx,
        &cancel,
    )
    .unwrap();
    assert_eq!(store.selected().unwrap().username, "integration-user");
    assert!(profile::credentials_path(&data, first).is_file());
    let selected = store.selected().unwrap().nm_uuid.to_string();
    // Avoid a desktop password prompt: use certificate authentication for the cancellation test.
    run_nmcli(
        &[
            "connection",
            "modify",
            "uuid",
            &selected,
            "+vpn.data",
            "connection-type=tls",
        ],
        10,
        None,
    )
    .unwrap();
    run_nmcli(
        &[
            "connection",
            "modify",
            "uuid",
            &selected,
            "+vpn.data",
            &format!("cert={},key={}", ca.display(), key.display()),
        ],
        10,
        None,
    )
    .unwrap();
    let flag = cancel.clone();
    let selected_uuid = store.selected().unwrap().nm_uuid;
    let cancellation = thread::spawn(move || {
        wait_for_activation(selected_uuid);
        flag.store(true, Ordering::Relaxed);
    });
    handle_request(VpnRequest::Connect, &data, &mut store, &tx, &cancel).unwrap();
    cancellation.join().unwrap();
    assert!(cancel.load(Ordering::Relaxed));
    handle_request(VpnRequest::Disconnect, &data, &mut store, &tx, &cancel).unwrap();
    let active = run_nmcli(&["-g", "UUID", "connection", "show", "--active"], 8, None).unwrap();
    assert!(!active.lines().any(|s| s == selected));
    // Exercise closing during activation through the actual worker queue.
    let backend = VpnBackend::start(data.clone());
    backend.requests.send(VpnRequest::Connect).unwrap();
    wait_for_activation(selected_uuid);
    backend.cancel.store(true, Ordering::Relaxed);
    backend.requests.send(VpnRequest::Shutdown).unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let event = backend
            .events
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("shutdown did not finish");
        match event {
            VpnEvent::ShutdownComplete => break,
            VpnEvent::ShutdownFailed(error) => panic!("shutdown failed: {error}"),
            _ => {}
        }
    }
    drop(backend);
    assert!(
        !active_uuids()
            .unwrap()
            .contains(&store.selected().unwrap().nm_uuid)
    );
    // Application-level exit must also join cleanup when no UI event loop remains.
    let backend = VpnBackend::start(data.clone());
    backend.requests.send(VpnRequest::Connect).unwrap();
    wait_for_activation(selected_uuid);
    drop(backend);
    assert!(
        !active_uuids()
            .unwrap()
            .contains(&store.selected().unwrap().nm_uuid)
    );
    assert_eq!(profile::load(&data).unwrap().profiles.len(), 2);
    for (uuid, before) in base_networks.iter().zip(baseline) {
        assert_eq!(
            snapshot(uuid),
            before,
            "The base connection routes/DNS changed"
        );
    }
    while store.selected().is_some() {
        let id = store.selected().unwrap().id;
        handle_request(VpnRequest::Remove(id), &data, &mut store, &tx, &cancel).unwrap();
    }
    assert!(profile::load(&data).unwrap().profiles.is_empty());
    cleanup.0.clear();
}
