//! Windows 固定内核强制退出后的实际协调器恢复验证；不操作桌面或用户系统代理。
use crate::{
    error::AppError,
    models::{PersistedConfiguration, ProxyProtocol, RuntimeMode},
    runtime::{BackendSession, ManagedRuntime, RuntimeBackend, RuntimePhase, SessionHealth},
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
    sync::{
        atomic::{AtomicU32, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

#[derive(Default)]
struct IsolatedProxy {
    port: Mutex<Option<u16>>,
    restores: AtomicUsize,
}
impl SystemProxyAdapter for Arc<IsolatedProxy> {
    fn enable(&self, port: u16) -> Result<(), AppError> {
        *self.port.lock().unwrap() = Some(port);
        Ok(())
    }
    fn restore_if_owned(&self) -> Result<(), AppError> {
        if self.port.lock().unwrap().take().is_some() {
            self.restores.fetch_add(1, Ordering::AcqRel);
        }
        Ok(())
    }
}

struct ObservedBackend {
    inner: SingBoxRuntimeBackend,
    pid: Arc<AtomicU32>,
}
impl RuntimeBackend for ObservedBackend {
    fn transition(
        &self,
        previous: Option<&BackendSession>,
        config: &PersistedConfiguration,
        mode: RuntimeMode,
        revision: u64,
    ) -> Result<Option<BackendSession>, AppError> {
        let session = self.inner.transition(previous, config, mode, revision)?;
        self.pid.store(
            session.as_ref().map_or(0, |s| s.process_id),
            Ordering::Release,
        );
        Ok(session)
    }
    fn confirm_transition(&self, previous: Option<&BackendSession>) {
        self.inner.confirm_transition(previous);
    }
    fn revert_transition(
        &self,
        candidate: Option<&BackendSession>,
        previous: Option<&BackendSession>,
    ) -> Result<(), AppError> {
        self.inner.revert_transition(candidate, previous)
    }
    fn reconcile_session(&self, session: &BackendSession) -> Result<bool, AppError> {
        self.inner.reconcile_session(session)
    }
}

#[test]
#[ignore = "显式真实固定 Windows 内核强制退出，隔离系统代理；要求报告路径"]
fn fixed_core_abrupt_exit_restores_isolated_network_and_explicit_retry(
) -> Result<(), Box<dyn Error>> {
    let output = std::env::var("ARCHITECTURE_CRASH_OUTPUT")?;
    let binary = crate::test_core::binary().ok_or("explicit SING_BOX_TEST_BIN required")?;
    let hash = hex::encode(Sha256::digest(std::fs::read(&binary)?));
    assert_eq!(hash, crate::sing_box_process::WINDOWS_AMD64_EXE_SHA256);
    let directory = tempfile::tempdir()?;
    let runtime_root = directory.path().join("runtime");
    let store = Arc::new(SqliteConfigurationStore::open(
        directory.path().join("crash.sqlite3"),
    )?);
    let credentials = Arc::new(MemoryCredentials::default());
    let proxy = Arc::new(IsolatedProxy::default());
    let pid = Arc::new(AtomicU32::new(0));
    let backend = ObservedBackend {
        inner: SingBoxRuntimeBackend::new(
            binary.into(),
            hash.clone(),
            runtime_root.clone(),
            Arc::new(credentials.clone()),
            Box::new(proxy.clone()),
        ),
        pid: pid.clone(),
    };
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
    let upstream = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    let profile = service.save_profile(ProfileInput {
        id: None,
        name: "isolated crash probe".into(),
        protocol: ProxyProtocol::Http,
        host: "127.0.0.1".into(),
        port: upstream.local_addr()?.port(),
        authentication_enabled: false,
        enabled: true,
        credential: None,
    })?;
    service.select_profile(Some(profile.id))?;
    let running = service.request_mode(crate::models::TEST_RULES_MODE)?;
    assert_eq!(running.session_health, SessionHealth::Healthy);
    let owned_pid = pid.load(Ordering::Acquire);
    assert_ne!(owned_pid, 0);
    assert_ne!(owned_pid, std::process::id());
    let port = proxy.port.lock().unwrap().expect("owned listener");
    // PID 来自本次 backend 返回的实际会话；只终止本测试创建的内核。
    let terminated = std::process::Command::new("taskkill")
        .args(["/PID", &owned_pid.to_string(), "/F"])
        .output()?;
    assert!(terminated.status.success(), "owned core termination failed");
    let started = Instant::now();
    let recovered = loop {
        // 与 runtime_events 相同的 250ms 协调器检查周期，不发送重启操作。
        let snapshot = service.runtime_snapshot();
        if snapshot.session_health == SessionHealth::Exited {
            break snapshot;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "exit not detected"
        );
        std::thread::sleep(Duration::from_millis(250));
    };
    let exit_detection_ms = started.elapsed().as_secs_f64() * 1000.;
    assert_eq!(recovered.phase, RuntimePhase::Failed);
    assert_eq!(recovered.selected_mode, crate::models::TEST_RULES_MODE);
    assert_eq!(recovered.applied_mode, None);
    assert!(!recovered.system_proxy_enabled);
    assert!(proxy.port.lock().unwrap().is_none());
    assert_eq!(proxy.restores.load(Ordering::Acquire), 1);
    assert!(std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(250)
    )
    .is_err());
    for _ in 0..3 {
        service.runtime_snapshot();
    }
    assert_eq!(proxy.restores.load(Ordering::Acquire), 1);
    assert_eq!(store.load_mode()?, crate::models::TEST_RULES_MODE);
    assert_eq!(recovered.configuration_revision, store.recovery_revision()?);
    let retried = service.request_mode(crate::models::TEST_RULES_MODE)?;
    assert_eq!(retried.phase, RuntimePhase::Running);
    assert_eq!(retried.session_health, SessionHealth::Healthy);
    assert!(retried.revision > recovered.revision);
    let restarted_pid = pid.load(Ordering::Acquire);
    assert_ne!(restarted_pid, owned_pid);
    service.stop_runtime()?;
    assert!(proxy.port.lock().unwrap().is_none());
    let directories = if runtime_root.exists() {
        std::fs::read_dir(runtime_root)?.count()
    } else {
        0
    };
    assert_eq!(directories, 0);
    std::fs::write(
        output,
        serde_json::to_vec_pretty(&serde_json::json!({
            "status":"passed", "platform":"Windows x64", "core_sha256":hash,
            "owned_core_pid":owned_pid, "explicit_retry_core_pid":restarted_pid,
        "exit_detection_ms":exit_detection_ms,
            "restores_after_exit":1, "runtime_directories_after_stop":directories,
            "native_ui":false, "system_proxy":"isolated adapter",
            "semantics":"automatic network restoration on reconcile; explicit retry starts new core"
        }))?,
    )?;
    Ok(())
}
