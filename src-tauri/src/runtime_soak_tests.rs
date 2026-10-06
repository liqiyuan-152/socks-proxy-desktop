//! 显式真实内核长时验证：临时配置、合成上游与代理适配器，不接管用户网络。
use crate::{
    error::AppError,
    models::{ProxyProtocol, RoutingRule, RuleAction, RuleMatcher, RuntimeMode},
    runtime::{ManagedRuntime, RuntimePhase, SessionHealth},
    runtime_session::SessionLease,
    services::{
        application_service::tests::{MemoryCredentials, MemoryStartup},
        ApplicationService, ProfileInput,
    },
    sing_box_backend::SingBoxRuntimeBackend,
    store::{ConfigurationStore, SqliteConfigurationStore},
    system_proxy::SystemProxyAdapter,
};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

#[derive(Default)]
struct TestProxy(Mutex<Option<u16>>);
impl SystemProxyAdapter for Arc<TestProxy> {
    fn enable(&self, port: u16) -> Result<(), AppError> {
        *self.0.lock().expect("proxy lock") = Some(port);
        Ok(())
    }
    fn restore_if_owned(&self) -> Result<(), AppError> {
        *self.0.lock().expect("proxy lock") = None;
        Ok(())
    }
}

struct Upstream {
    port: u16,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<std::io::Result<()>>>,
}
impl Upstream {
    fn new() -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let worker = std::thread::spawn(move || {
            while !stopping.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        // 部分平台的 accept 会继承监听器的非阻塞模式。
                        stream.set_nonblocking(false)?;
                        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
                        stream.set_write_timeout(Some(Duration::from_secs(3)))?;
                        let request = header(&mut stream)?;
                        if !request.starts_with("CONNECT soak.invalid:443 ") {
                            return Err(std::io::Error::other("unexpected upstream target"));
                        }
                        stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")?;
                        let mut payload = [0; 5];
                        stream.read_exact(&mut payload)?;
                        if payload != *b"probe" {
                            return Err(std::io::Error::other("unexpected payload"));
                        }
                        stream.write_all(b"ready")?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => return Err(error),
                }
            }
            Ok(())
        });
        Ok(Self {
            port,
            stop,
            worker: Some(worker),
        })
    }
    fn finish(&mut self) -> Result<(), Box<dyn Error>> {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| "upstream worker panicked")??;
        }
        Ok(())
    }
}
impl Drop for Upstream {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            eprintln!("soak fixture cleanup: {error}");
        }
    }
}

fn header(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        if bytes.len() >= 8192 {
            return Err(std::io::Error::other("header limit exceeded"));
        }
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        bytes.push(byte[0]);
    }
    String::from_utf8(bytes).map_err(std::io::Error::other)
}

fn probe(proxy: &TestProxy) -> Result<(), Box<dyn Error>> {
    let port = proxy
        .0
        .lock()
        .map_err(|_| "proxy lock poisoned")?
        .ok_or("no active port")?;
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all(b"CONNECT soak.invalid:443 HTTP/1.1\r\nHost: soak.invalid:443\r\n\r\n")?;
    assert!(header(&mut stream)?.starts_with("HTTP/1.1 200"));
    stream.write_all(b"probe")?;
    let mut result = [0; 5];
    stream.read_exact(&mut result)?;
    assert_eq!(result, *b"ready");
    Ok(())
}

#[test]
#[ignore = "显式设置真实固定内核与报告路径；默认运行 30 分钟，不接管用户系统代理"]
fn real_core_configuration_and_transport_soak() -> Result<(), Box<dyn Error>> {
    let seconds: u64 = std::env::var("ARCHITECTURE_SOAK_SECONDS")
        .unwrap_or_else(|_| "1800".into())
        .parse()?;
    if !(1..=86400).contains(&seconds) {
        return Err("soak duration outside supported range".into());
    }
    let output = std::env::var("ARCHITECTURE_SOAK_OUTPUT")?;
    let binary = crate::test_core::binary().ok_or("explicit SING_BOX_TEST_BIN is required")?;
    let version = std::process::Command::new(&binary)
        .arg("version")
        .output()?;
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout)?.lines().next(),
        Some("sing-box version 1.14.1")
    );
    let checksum = hex::encode(Sha256::digest(std::fs::read(&binary)?));
    #[cfg(windows)]
    assert_eq!(checksum, crate::sing_box_process::WINDOWS_AMD64_EXE_SHA256);
    let directory = tempfile::tempdir()?;
    let runtime_root = directory.path().join("runtime");
    let store = Arc::new(SqliteConfigurationStore::open(
        directory.path().join("soak.sqlite3"),
    )?);
    let credentials = Arc::new(MemoryCredentials::default());
    let proxy = Arc::new(TestProxy::default());
    let backend = SingBoxRuntimeBackend::new(
        binary.into(),
        checksum.clone(),
        runtime_root.clone(),
        Arc::new(credentials.clone()),
        Box::new(proxy.clone()),
    );
    let runtime = ManagedRuntime::from_lease(
        store.load()?,
        Box::new(backend),
        SessionLease::acquire(&uuid::Uuid::new_v4().to_string())?,
        RuntimeMode::Direct,
    )?;
    let service = ApplicationService::new(
        Box::new(store.clone()),
        Box::new(credentials),
        Box::new(Arc::new(MemoryStartup::default())),
        Box::new(runtime),
    );
    let mut upstream = Upstream::new()?;
    let profile_input = |id, name| ProfileInput {
        id,
        name,
        protocol: ProxyProtocol::Http,
        host: "127.0.0.1".into(),
        port: upstream.port,
        authentication_enabled: false,
        enabled: true,
        credential: None,
    };
    let id = service.save_profile(profile_input(None, "Soak".into()))?.id;
    service.select_profile(Some(id.clone()))?;
    service.replace_rules(vec![RoutingRule {
        id: "soak-rule".into(),
        name: "Soak".into(),
        matcher: RuleMatcher::Domain,
        target: "soak.invalid".into(),
        port_start: None,
        port_end: None,
        action: RuleAction::Proxy,
        proxy_profile_id: Some(id.clone()),
        enabled: true,
    }])?;
    service.request_mode(RuntimeMode::Global)?;
    let started = Instant::now();
    let mut probes = 0;
    let mut transitions = 0;
    let mut commits = 0;
    let mut last_maintenance = Instant::now();
    let mut max_runtime_dirs = 0;
    while started.elapsed() < Duration::from_secs(seconds) {
        if probes == 0 || last_maintenance.elapsed() >= Duration::from_secs(30) {
            let mode = if transitions % 2 == 0 {
                crate::models::TEST_RULES_MODE
            } else {
                RuntimeMode::Global
            };
            service.request_mode(mode)?;
            transitions += 1;
            service.save_profile(profile_input(Some(id.clone()), format!("Soak {commits}")))?;
            commits += 1;
            if transitions % 4 == 0 {
                service.stop_runtime()?;
                assert!(proxy.0.lock().map_err(|_| "proxy lock poisoned")?.is_none());
                service.request_mode(mode)?;
            }
            last_maintenance = Instant::now();
            println!("SOAK_PROGRESS seconds={:.1} probes={probes} transitions={transitions} commits={commits}", started.elapsed().as_secs_f64());
            std::io::stdout().flush()?;
        }
        probe(&proxy)?;
        probes += 1;
        let snapshot = service.runtime_snapshot();
        assert_eq!(snapshot.phase, RuntimePhase::Running);
        assert_eq!(snapshot.session_health, SessionHealth::Healthy);
        assert_eq!(snapshot.configuration_revision, store.recovery_revision()?);
        let dirs = std::fs::read_dir(&runtime_root)?.count();
        max_runtime_dirs = max_runtime_dirs.max(dirs);
        assert!(dirs <= 1, "stale core runtime directory leaked");
        std::thread::sleep(Duration::from_secs(1));
    }
    service.stop_runtime()?;
    assert!(proxy.0.lock().map_err(|_| "proxy lock poisoned")?.is_none());
    assert_eq!(std::fs::read_dir(&runtime_root)?.count(), 0);
    assert_eq!(
        store.load()?.profiles[0].name,
        format!("Soak {}", commits - 1)
    );
    upstream.finish()?;
    let report = serde_json::json!({ "status": "passed", "platform": std::env::consts::OS,
        "core_version": "1.14.1", "core_sha256": checksum, "duration_seconds": started.elapsed().as_secs_f64(),
        "requested_seconds": seconds, "probes": probes, "mode_transitions": transitions,
        "metadata_commits": commits, "maximum_runtime_directories": max_runtime_dirs,
        "runtime_directories_after_stop": 0, "system_proxy": "isolated adapter", "native_ui": false });
    std::fs::write(output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
