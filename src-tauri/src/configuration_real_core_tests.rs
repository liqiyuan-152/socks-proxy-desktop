//! Fixed-core integration: observe actual HTTP proxy authentication after
//! credential-only commits; configuration rendering alone is insufficient.
use super::*;
use crate::{
    runtime::ManagedRuntime, runtime_session::SessionLease,
    sing_box_backend::SingBoxRuntimeBackend, system_proxy::SystemProxyAdapter,
};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

#[derive(Default)]
struct ProxyState {
    port: Mutex<Option<u16>>,
    fail_next: AtomicBool,
}
impl SystemProxyAdapter for Arc<ProxyState> {
    fn enable(&self, port: u16) -> Result<(), AppError> {
        if self.fail_next.swap(false, Ordering::SeqCst) {
            return Err(AppError::unavailable("synthetic proxy commit failure"));
        }
        *self.port.lock().unwrap() = Some(port);
        Ok(())
    }
    fn restore_if_owned(&self) -> Result<(), AppError> {
        *self.port.lock().unwrap() = None;
        Ok(())
    }
}

fn read_header(stream: &mut TcpStream) -> String {
    // Windows accepted sockets can inherit the listener's nonblocking mode.
    // read_exact needs blocking reads with the bounded timeout below.
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        assert!(header.len() < 8192, "HTTP header exceeded fixture limit");
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        header.push(byte[0]);
    }
    String::from_utf8(header).unwrap()
}

#[test]
fn header_reader_waits_for_delayed_data_on_nonblocking_stream() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    client.set_nonblocking(true).unwrap();
    let (mut server, _) = listener.accept().unwrap();
    let writer = std::thread::spawn(move || {
        server.write_all(b"HTTP/1.1 200").unwrap();
        std::thread::sleep(Duration::from_millis(50));
        server.write_all(b" OK\r\n\r\n").unwrap();
    });
    assert_eq!(read_header(&mut client), "HTTP/1.1 200 OK\r\n\r\n");
    writer.join().unwrap();
}

fn probe(proxy: &ProxyState, observed: &mpsc::Receiver<String>, expected: &str) {
    let port = proxy.port.lock().unwrap().unwrap();
    let mut client = TcpStream::connect(("127.0.0.1", port)).unwrap();
    client
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    client
        .write_all(b"CONNECT example.com:443 HTTP/1.1\r\nHost: example.com:443\r\n\r\n")
        .unwrap();
    let request = observed.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(request.starts_with("CONNECT example.com:443 "));
    assert!(request
        .lines()
        .any(|line| line.eq_ignore_ascii_case(&format!("Proxy-Authorization: Basic {expected}"))));
    assert!(read_header(&mut client).starts_with("HTTP/1.1 200"));
}

fn input(id: Option<String>, upstream_port: u16, username: &str, password: &str) -> ProfileInput {
    ProfileInput {
        id,
        name: "Authenticated".into(),
        protocol: ProxyProtocol::Http,
        host: "127.0.0.1".into(),
        port: upstream_port,
        authentication_enabled: true,
        enabled: true,
        credential: Some(CredentialUpdate::Replace {
            username: username.into(),
            password: password.into(),
        }),
    }
}

#[test]
fn fixed_core_applies_password_username_import_and_preserves_credentials_after_failure() {
    let Some(binary) = crate::test_core::binary() else {
        return;
    };
    let version = std::process::Command::new(&binary)
        .arg("version")
        .output()
        .unwrap();
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).unwrap().lines().next(),
        Some("sing-box version 1.14.1")
    );
    let checksum = hex::encode(Sha256::digest(std::fs::read(&binary).unwrap()));
    let directory = tempfile::tempdir().unwrap();
    let store = Arc::new(crate::store::SqliteConfigurationStore::open_in_memory().unwrap());
    let credentials = Arc::new(MemoryCredentials::default());
    let proxy = Arc::new(ProxyState::default());
    let backend = SingBoxRuntimeBackend::new(
        binary.into(),
        checksum,
        directory.path().join("runtime"),
        Arc::new(credentials.clone()),
        Box::new(proxy.clone()),
    );
    let runtime = ManagedRuntime::from_lease(
        store.load().unwrap(),
        Box::new(backend),
        SessionLease::acquire(&Uuid::new_v4().to_string()).unwrap(),
        RuntimeMode::Direct,
    )
    .unwrap();
    let service = ApplicationService::new(
        Box::new(store.clone()),
        Box::new(credentials),
        Box::new(Arc::new(MemoryStartup::default())),
        Box::new(runtime),
    );
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let upstream_port = listener.local_addr().unwrap().port();
    let (sender, observed) = mpsc::channel();
    let server = std::thread::spawn(move || {
        // A deadline ensures a failed assertion cannot leave an endless accept.
        let deadline = std::time::Instant::now() + Duration::from_secs(90);
        for _ in 0..5 {
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            std::time::Instant::now() < deadline,
                            "fixture accept timed out"
                        );
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("fixture accept failed: {error}"),
                }
            };
            sender.send(read_header(&mut stream)).unwrap();
            stream
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .unwrap();
        }
    });
    let id = service
        .save_profile(input(None, upstream_port, "alice", "old"))
        .unwrap()
        .id;
    service.select_profile(Some(id.clone())).unwrap();
    service.request_mode(RuntimeMode::Global).unwrap();
    probe(&proxy, &observed, "YWxpY2U6b2xk"); // alice:old, synthetic values only.
    let portable = service.export().unwrap();
    let mut revision = store.recovery_revision().unwrap();
    for (username, password, expected) in [
        ("alice", "updated", "YWxpY2U6dXBkYXRlZA=="),
        ("bob", "updated", "Ym9iOnVwZGF0ZWQ="),
    ] {
        service
            .save_profile(input(Some(id.clone()), upstream_port, username, password))
            .unwrap();
        assert_eq!(service.export().unwrap(), portable);
        revision += 1;
        assert_eq!(store.recovery_revision().unwrap(), revision);
        probe(&proxy, &observed, expected);
    }
    service
        .import(
            &portable,
            HashMap::from([(
                id.clone(),
                CredentialUpdate::Replace {
                    username: "bob".into(),
                    password: "imported".into(),
                },
            )]),
        )
        .unwrap();
    revision += 1;
    probe(&proxy, &observed, "Ym9iOmltcG9ydGVk");
    let committed = store.load().unwrap();
    let previous_port = *proxy.port.lock().unwrap();
    proxy.fail_next.store(true, Ordering::SeqCst);
    assert!(service
        .save_profile(input(Some(id), upstream_port, "rejected", "candidate"))
        .is_err());
    assert_eq!(store.load().unwrap(), committed);
    assert_eq!(store.recovery_revision().unwrap(), revision);
    assert_eq!(*proxy.port.lock().unwrap(), previous_port);
    probe(&proxy, &observed, "Ym9iOmltcG9ydGVk");
    service.stop_runtime().unwrap();
    assert!(proxy.port.lock().unwrap().is_none());
    assert_eq!(
        std::fs::read_dir(directory.path().join("runtime"))
            .unwrap()
            .count(),
        0
    );
    server.join().unwrap();
}
