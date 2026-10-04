//! SQLite、应用 facade、凭据与异步测速注册表的组合生命周期回归。
use crate::{
    credentials::CredentialUpdate,
    error::AppError,
    latency_tasks::{
        LatencyExecutor, LatencyInput, LatencyResult, LatencyTaskRegistry, LatencyTaskState,
    },
    models::ProxyProtocol,
    services::{
        application_service::tests::{FakeRuntime, MemoryCredentials, MemoryStartup},
        ApplicationService, ProfileInput,
    },
    store::SqliteConfigurationStore,
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};

struct Executor {
    started: mpsc::Sender<(String, u64)>,
    result: Mutex<mpsc::Receiver<Result<LatencyResult, AppError>>>,
    calls: AtomicUsize,
}
impl LatencyExecutor for Executor {
    fn run(
        &self,
        input: LatencyInput,
        cancelled: Arc<AtomicBool>,
    ) -> Result<LatencyResult, AppError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        assert!(input
            .credential
            .as_ref()
            .is_some_and(|secret| secret.password == "fixture-secret"));
        self.started
            .send((input.profile_id, input.configuration_revision))
            .map_err(|_| AppError::unavailable("测试启动信号已结束"))?;
        loop {
            if cancelled.load(Ordering::Acquire) {
                return Ok(LatencyResult { latency_ms: 999 });
            }
            match self
                .result
                .lock()
                .map_err(|_| AppError::unavailable("测试信号锁不可用"))?
                .recv_timeout(Duration::from_millis(10))
            {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => return Err(AppError::unavailable("测试结果信号已结束")),
            }
        }
    }
}
fn input(id: Option<String>, name: &str, credential: CredentialUpdate) -> ProfileInput {
    ProfileInput {
        id,
        name: name.into(),
        protocol: ProxyProtocol::Socks5,
        host: "127.0.0.1".into(),
        port: 1080,
        enabled: true,
        authentication_enabled: true,
        credential: Some(credential),
    }
}
fn wait_state(
    service: &ApplicationService,
    subscription: &str,
    state: LatencyTaskState,
) -> Result<(), AppError> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if service.latency_task_snapshot(subscription)?.state == state {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(AppError::unavailable("测速生命周期未达到预期状态"));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn facade_latency_lifecycle_shares_jobs_preserves_errors_and_invalidates_committed_versions(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let store = Arc::new(SqliteConfigurationStore::open(
        directory.path().join("configuration.sqlite3"),
    )?);
    let (started_tx, started_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    let executor = Arc::new(Executor {
        started: started_tx,
        result: Mutex::new(result_rx),
        calls: AtomicUsize::new(0),
    });
    let service = ApplicationService::new(
        Box::new(store),
        Box::new(Arc::new(MemoryCredentials::default())),
        Box::new(Arc::new(MemoryStartup::default())),
        Box::new(Arc::new(FakeRuntime::default())),
    )
    .with_latency_tasks(LatencyTaskRegistry::new(executor.clone()));
    let profile = service.save_profile(input(
        None,
        "Primary",
        CredentialUpdate::Replace {
            username: "fixture-user".into(),
            password: "fixture-secret".into(),
        },
    ))?;
    let before = service.runtime_snapshot();
    let first = service.start_latency_task(&profile.id)?;
    let second = service.start_latency_task(&profile.id)?;
    assert_eq!(first.task.task_id, second.task.task_id);
    let captured = started_rx.recv_timeout(Duration::from_secs(3))?;
    assert_eq!(
        captured,
        (profile.id.clone(), profile.configuration_revision)
    );
    service.release_latency_task(&first.subscription_id)?;
    assert!(service
        .latency_task_snapshot(&first.subscription_id)
        .is_err());
    result_tx.send(Ok(LatencyResult { latency_ms: 42 }))?;
    wait_state(
        &service,
        &second.subscription_id,
        LatencyTaskState::Succeeded,
    )?;
    assert_eq!(
        service
            .latency_task_snapshot(&second.subscription_id)?
            .result
            .map(|r| r.latency_ms),
        Some(42)
    );
    assert_eq!(executor.calls.load(Ordering::Relaxed), 1);
    service.release_latency_task(&second.subscription_id)?;

    let failed = service.start_latency_task(&profile.id)?;
    started_rx.recv_timeout(Duration::from_secs(3))?;
    result_tx.send(Err(AppError::unavailable("fixture failure")))?;
    wait_state(&service, &failed.subscription_id, LatencyTaskState::Failed)?;
    let snapshot = service.latency_task_snapshot(&failed.subscription_id)?;
    assert!(snapshot.result.is_none());
    assert_eq!(
        snapshot.error.map(|e| e.message).as_deref(),
        Some("fixture failure")
    );
    service.release_latency_task(&failed.subscription_id)?;

    let stale = service.start_latency_task(&profile.id)?;
    started_rx.recv_timeout(Duration::from_secs(3))?;
    let updated = service.save_profile(input(
        Some(profile.id.clone()),
        "Edited",
        CredentialUpdate::Preserve,
    ))?;
    assert!(updated.configuration_revision > stale.task.configuration_revision);
    wait_state(
        &service,
        &stale.subscription_id,
        LatencyTaskState::Cancelled,
    )?;
    let snapshot = service.latency_task_snapshot(&stale.subscription_id)?;
    assert!(snapshot.result.is_none() && snapshot.error.is_none());
    service.release_latency_task(&stale.subscription_id)?;
    service.release_latency_task(&stale.subscription_id)?;
    assert!(service
        .latency_task_snapshot(&stale.subscription_id)
        .is_err());
    let after = service.runtime_snapshot();
    assert_eq!(before.phase, after.phase);
    assert_eq!(before.applied_mode, after.applied_mode);
    assert_eq!(before.system_proxy_enabled, after.system_proxy_enabled);
    assert_eq!(executor.calls.load(Ordering::Relaxed), 3);
    Ok(())
}
