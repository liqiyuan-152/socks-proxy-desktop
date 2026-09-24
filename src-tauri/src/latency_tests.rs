use super::*;
use crate::models::{PersistedConfiguration, ProxyProfile, ProxyProtocol};
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    sync::Mutex,
    thread,
};

struct Store(PersistedConfiguration);
impl ConfigurationStore for Store {
    fn load(&self) -> Result<PersistedConfiguration, AppError> {
        Ok(self.0.clone())
    }
    fn save(&self, _: &PersistedConfiguration) -> Result<(), AppError> {
        Ok(())
    }
    fn load_mode(&self) -> Result<RuntimeMode, AppError> {
        Ok(RuntimeMode::Direct)
    }
    fn save_mode(&self, _: RuntimeMode) -> Result<(), AppError> {
        Ok(())
    }
}

struct Credentials(Mutex<usize>);
impl CredentialStore for Credentials {
    fn get(&self, _: &str) -> Result<Option<crate::credentials::ProxyCredential>, AppError> {
        *self.0.lock().unwrap() += 1;
        Ok(Some(crate::credentials::ProxyCredential {
            username: "alice".into(),
            password: "secret".into(),
        }))
    }
    fn replace(&self, _: &str, _: &str, _: &str) -> Result<(), AppError> {
        Ok(())
    }
    fn delete(&self, _: &str) -> Result<(), AppError> {
        Ok(())
    }
}

fn configuration(authenticated: bool) -> PersistedConfiguration {
    let mut config = PersistedConfiguration::default();
    config.profiles.push(ProxyProfile {
        id: "test".into(),
        name: "Test".into(),
        protocol: ProxyProtocol::Http,
        host: "127.0.0.1".into(),
        port: 1080,
        authentication_enabled: authenticated,
        credential_ref: authenticated.then(|| "test".into()),
        enabled: true,
    });
    config
}

#[test]
fn statuses_only_accept_2xx() {
    for status in [200, 204, 299] {
        assert!(validate_status(status).is_ok());
    }
    for status in [301, 401, 404, 500] {
        assert!(validate_status(status).is_err());
    }
}

#[test]
fn reads_credentials_without_switching_selection_or_system_proxy() {
    let store = Arc::new(Store(configuration(true)));
    let credentials = Arc::new(Credentials(Mutex::new(0)));
    let temp = tempfile::tempdir().unwrap();
    let tester = LatencyTester::new(
        store.clone(),
        credentials.clone(),
        temp.path().join("missing-core"),
        "checksum".into(),
        temp.path().join("runtime"),
    );
    assert!(tester.test("missing").is_err());
    assert_eq!(*credentials.0.lock().unwrap(), 0);
    assert!(tester.test("test").is_err());
    assert_eq!(*credentials.0.lock().unwrap(), 1);
    assert_eq!(store.load().unwrap().active_profile_id, None);
    assert!(!temp.path().join("runtime").exists());
}

#[test]
fn reports_proxy_failure_without_exposing_target_or_credentials() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0u8; 1024];
        let count = stream.read(&mut request).unwrap();
        assert!(String::from_utf8_lossy(&request[..count]).contains("CONNECT example.org:443"));
        stream
            .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\nContent-Length: 0\r\n\r\n")
            .unwrap();
    });
    let error = probe("https://example.org/check", port).unwrap_err();
    assert_eq!(error.message, "代理连接或测试请求失败");
    assert!(!error.message.contains("example.org"));
    server.join().unwrap();
}

#[test]
fn measures_success_and_rejects_redirect_through_local_proxy() {
    for (status, success) in [("204 No Content", true), ("302 Found", false)] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 2048];
            let count = stream.read(&mut request).unwrap();
            assert!(String::from_utf8_lossy(&request[..count])
                .contains("GET http://example.org/check HTTP/1.1"));
            let response = format!(
                "HTTP/1.1 {status}\r\nLocation: http://example.org/other\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        assert_eq!(probe("http://example.org/check", port).is_ok(), success);
        server.join().unwrap();
    }
}

#[test]
fn timeout_reports_failure_and_does_not_return_a_measurement() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0u8; 1024];
        let count = stream.read(&mut request).unwrap();
        assert!(count > 0);
        thread::sleep(Duration::from_millis(200));
    });
    let error = probe_with_timeout("https://example.org/check", port, Duration::from_millis(50))
        .unwrap_err();
    assert_eq!(error.message, "代理测试超时（5 秒）");
    server.join().unwrap();
}

#[test]
fn temporary_core_is_cleaned_after_failed_request_without_changing_saved_selection() {
    let directory = tempfile::tempdir().unwrap();
    let binary = directory.path().join(if cfg!(windows) {
        "fake-core.exe"
    } else {
        "fake-core"
    });
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-sing-box.rs");
    let output = Command::new("rustc")
        .arg("--edition=2021")
        .arg(fixture)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(output.status.success());
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    std::io::copy(&mut std::fs::File::open(&binary).unwrap(), &mut hasher).unwrap();
    let store = Arc::new(Store(configuration(true)));
    let credentials = Arc::new(Credentials(Mutex::new(0)));
    let runtime_root = directory.path().join("runtime");
    let tester = LatencyTester::new(
        store.clone(),
        credentials.clone(),
        binary,
        hex::encode(hasher.finalize()),
        runtime_root.clone(),
    );
    assert!(tester.test("test").is_err());
    assert_eq!(*credentials.0.lock().unwrap(), 1);
    assert_eq!(store.load().unwrap().active_profile_id, None);
    assert_eq!(std::fs::read_dir(runtime_root).unwrap().count(), 0);
}
