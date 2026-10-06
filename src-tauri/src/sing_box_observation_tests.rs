use super::*;
use crate::core_control::CoreControlClient;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    time::{Duration, Instant},
};

#[test]
fn slow_control_request_does_not_lock_switch_or_stop_and_old_success_is_discarded() {
    let directory = tempfile::tempdir().unwrap();
    let (binary, checksum) = compile_fake_core(directory.path());
    // Candidate verification/startup may legitimately exceed the control timeout.
    // Use the controlled core for switch timing; fixed-core runs time stop only.
    let operations: &[bool] = if crate::test_core::binary().is_some() {
        &[true]
    } else {
        &[false, true]
    };
    for &stop in operations {
        let proxy = Arc::new(ProxyState::default());
        let backend = Arc::new(SingBoxRuntimeBackend::new(
            binary.clone(),
            checksum.clone(),
            directory.path().into(),
            Arc::new(Credentials),
            Box::new(proxy.clone()),
        ));
        let config = configuration();
        let first = backend
            .transition(None, &config, RuntimeMode::Global, 1)
            .unwrap()
            .unwrap();
        backend.confirm_transition(None);

        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let client =
            CoreControlClient::new(listener.local_addr().unwrap().port(), "test-key".into())
                .unwrap();
        backend
            .slots
            .lock()
            .unwrap()
            .active
            .as_mut()
            .unwrap()
            .process
            .replace_test_control_client(client);
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            entered_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            let body = b"{\"connections\":[],\"old\":true}";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(body).unwrap();
        });
        let reader = backend.clone();
        let response = std::thread::spawn(move || reader.active_connections_json());
        entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(
            backend.slots.try_lock().is_ok(),
            "control IO owns the session lock"
        );
        let started = Instant::now();
        let mode = if stop {
            RuntimeMode::Direct
        } else {
            crate::models::TEST_RULES_MODE
        };
        let (done_tx, done_rx) = mpsc::channel();
        let writer = backend.clone();
        let mutation = std::thread::spawn(move || {
            let result = writer.transition(Some(&first), &config, mode, 2);
            if result.is_ok() {
                writer.confirm_transition(Some(&first));
            }
            done_tx.send(result).unwrap();
        });
        let result = done_rx.recv_timeout(Duration::from_millis(900));
        release_tx.send(()).unwrap();
        assert!(
            result.is_ok(),
            "session mutation waited for control response"
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        let next = result.unwrap().unwrap();
        mutation.join().unwrap();
        let error = response.join().unwrap().unwrap_err();
        assert_eq!(error.message, "旧会话观测已失效");
        server.join().unwrap();
        if stop {
            assert!(next.is_none());
            assert_eq!(*proxy.port.lock().unwrap(), None);
        } else {
            assert!(next.is_some());
            assert!(backend.active_connections_json().unwrap()["connections"].is_array());
        }
    }
}

fn compile_fake_core(directory: &std::path::Path) -> (std::path::PathBuf, String) {
    if let Some(binary) = crate::test_core::binary() {
        let binary = std::path::PathBuf::from(binary);
        let checksum = hex::encode(Sha256::digest(std::fs::read(&binary).unwrap()));
        return (binary, checksum);
    }
    let binary = directory.join(if cfg!(windows) {
        "fake-sing-box.exe"
    } else {
        "fake-sing-box"
    });
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-sing-box.rs");
    assert!(std::process::Command::new("rustc")
        .args(["--edition=2021"])
        .arg(fixture)
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap()
        .success());
    let checksum = hex::encode(Sha256::digest(std::fs::read(&binary).unwrap()));
    (binary, checksum)
}

#[test]
fn fixed_core_failed_switch_retains_old_process_then_recovers_after_exit() {
    let Some(binary) = crate::test_core::binary() else {
        return;
    };
    let checksum = hex::encode(Sha256::digest(std::fs::read(&binary).unwrap()));
    let directory = tempfile::tempdir().unwrap();
    let proxy = Arc::new(ProxyState::default());
    let backend = SingBoxRuntimeBackend::new(
        binary.into(),
        checksum,
        directory.path().into(),
        Arc::new(Credentials),
        Box::new(proxy.clone()),
    );
    let config = configuration();
    let first = backend
        .transition(None, &config, RuntimeMode::Global, 1)
        .unwrap()
        .unwrap();
    backend.confirm_transition(None);
    let first_port = *proxy.port.lock().unwrap();
    proxy.reject_next.store(true, Ordering::SeqCst);
    assert!(backend
        .transition(Some(&first), &config, crate::models::TEST_RULES_MODE, 2)
        .is_err());
    assert_eq!(*proxy.port.lock().unwrap(), first_port);
    assert!(backend.reconcile_session(&first).unwrap());
    // Kill the retained actual core, leaving the committed slot for the
    // production health check to detect and restore.
    backend
        .slots
        .lock()
        .unwrap()
        .active
        .as_mut()
        .unwrap()
        .process
        .stop();
    assert!(!backend.reconcile_session(&first).unwrap());
    assert!(proxy.port.lock().unwrap().is_none());
    assert!(backend.slots.lock().unwrap().active.is_none());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    assert!(!backend.reconcile_session(&first).unwrap());
}

#[test]
fn global_core_starts_without_unrelated_exit_secret_but_rules_rejects_it() {
    let directory = tempfile::tempdir().unwrap();
    let (binary, checksum) = compile_fake_core(directory.path());
    let proxy = Arc::new(ProxyState::default());
    let backend = SingBoxRuntimeBackend::new(
        binary,
        checksum,
        directory.path().into(),
        Arc::new(Credentials),
        Box::new(proxy),
    );
    let mut config = configuration();
    let mut unrelated = config.profiles[0].clone();
    unrelated.id = "missing-credential".into();
    unrelated.name = "Unrelated".into();
    unrelated.authentication_enabled = true;
    unrelated.credential_ref = Some(unrelated.id.clone());
    config.profiles.push(unrelated);
    let started = backend
        .transition(None, &config, RuntimeMode::Global, 1)
        .unwrap()
        .unwrap();
    backend.confirm_transition(None);
    let rendered = {
        let slots = backend.slots.lock().unwrap();
        let raw =
            std::fs::read_to_string(slots.active.as_ref().unwrap().process.config_path()).unwrap();
        serde_json::from_str::<serde_json::Value>(&raw).unwrap()
    };
    assert_eq!(rendered["outbounds"].as_array().unwrap().len(), 2);
    let error = backend
        .transition(Some(&started), &config, crate::models::TEST_RULES_MODE, 2)
        .unwrap_err();
    assert_eq!(error.fields[0].field, "profiles[1].credential");
    assert!(backend.reconcile_session(&started).unwrap());
    assert_eq!(
        backend
            .slots
            .lock()
            .unwrap()
            .active
            .as_ref()
            .unwrap()
            .identity,
        started
    );
}
