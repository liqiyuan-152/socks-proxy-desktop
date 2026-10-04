use super::*;
use crate::models::{PersistedConfiguration, ProxyProfile, ProxyProtocol};
use crate::{
    credentials::ProxyCredential,
    models::{RetentionPolicy, RuleAction, RuleMatcher, RuntimeMode},
    runtime::{RuntimePhase, RuntimeSnapshot},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

pub(crate) struct MemoryStore {
    value: Mutex<PersistedConfiguration>,
    mode: Mutex<RuntimeMode>,
    pub(crate) fail_save: Mutex<bool>,
    recovery: Mutex<(u64, Option<crate::configuration_recovery::RecoveryRecord>)>,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self {
            value: Mutex::new(PersistedConfiguration::default()),
            mode: Mutex::new(RuntimeMode::Direct),
            fail_save: Mutex::new(false),
            recovery: Mutex::new((0, None)),
        }
    }
}

#[path = "../configuration_memory_recovery.rs"]
mod memory_recovery;

#[path = "../configuration_real_core_tests.rs"]
mod real_core_tests;

#[path = "../configuration_durable_tests.rs"]
mod durable_tests;

#[path = "../configuration_staging_tests.rs"]
mod staging_tests;

#[derive(Default)]
pub(crate) struct MemoryCredentials(Mutex<HashMap<String, ProxyCredential>>);

impl CredentialStore for Arc<MemoryCredentials> {
    fn get(&self, id: &str) -> Result<Option<ProxyCredential>, AppError> {
        Ok(self.0.lock().unwrap().get(id).map(|value| ProxyCredential {
            username: value.username.clone(),
            password: value.password.clone(),
        }))
    }
    fn replace(&self, id: &str, username: &str, password: &str) -> Result<(), AppError> {
        self.0.lock().unwrap().insert(
            id.into(),
            ProxyCredential {
                username: username.into(),
                password: password.into(),
            },
        );
        Ok(())
    }
    fn delete(&self, id: &str) -> Result<(), AppError> {
        self.0.lock().unwrap().remove(id);
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct MemoryStartup(Mutex<bool>);

impl StartupAdapter for Arc<MemoryStartup> {
    fn is_enabled(&self) -> Result<bool, AppError> {
        Ok(*self.0.lock().unwrap())
    }
    fn set_enabled(&self, enabled: bool) -> Result<(), AppError> {
        *self.0.lock().unwrap() = enabled;
        Ok(())
    }
    fn read_entry(&self) -> Result<Option<crate::configuration_recovery::StartupEntry>, AppError> {
        Ok((*self.0.lock().unwrap()).then(|| self.expected_entry().unwrap()))
    }
    fn expected_entry(&self) -> Result<crate::configuration_recovery::StartupEntry, AppError> {
        Ok(crate::configuration_recovery::StartupEntry {
            value_type: 1,
            bytes: vec![65, 0, 0, 0],
        })
    }
    fn write_entry(
        &self,
        entry: Option<&crate::configuration_recovery::StartupEntry>,
    ) -> Result<(), AppError> {
        *self.0.lock().unwrap() = entry.is_some();
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct FakeRuntime {
    pub(crate) reject_next: Mutex<bool>,
    pub(crate) committed_active_ids: Mutex<Vec<Option<String>>>,
    recoveries: Mutex<usize>,
}

impl RuntimeCoordinator for Arc<FakeRuntime> {
    fn snapshot(&self) -> RuntimeSnapshot {
        RuntimeSnapshot {
            configuration_revision: 0,
            runtime_plan_revision: 0,
            revision: 0,
            selected_mode: RuntimeMode::Direct,
            desired_mode: RuntimeMode::Direct,
            applied_mode: Some(RuntimeMode::Direct),
            phase: RuntimePhase::Stopped,
            active_profile_id: None,
            runtime_uptime_ms: None,
            system_proxy_enabled: false,
            tun_enabled: false,
            coverage: crate::runtime::TrafficCoverage::None,
            session_health: crate::runtime::SessionHealth::Healthy,
            last_operation: crate::runtime::OperationResult::default(),
            last_error: None,
        }
    }
    fn request_mode(&self, _: RuntimeMode) -> Result<RuntimeSnapshot, AppError> {
        Ok(self.snapshot())
    }
    fn stop(&self) -> Result<RuntimeSnapshot, AppError> {
        Ok(self.snapshot())
    }
    fn recover_network(&self) -> Result<RuntimeSnapshot, AppError> {
        *self.recoveries.lock().unwrap() += 1;
        Ok(self.snapshot())
    }
    fn apply_configuration(
        &self,
        _: &PersistedConfiguration,
        candidate: &PersistedConfiguration,
    ) -> Result<RuntimeSnapshot, AppError> {
        if std::mem::take(&mut *self.reject_next.lock().unwrap()) {
            return Err(AppError::unavailable("运行时无法提交"));
        }
        self.committed_active_ids
            .lock()
            .unwrap()
            .push(candidate.active_profile_id.clone());
        Ok(self.snapshot())
    }
    fn confirm_configuration(&self) -> RuntimeSnapshot {
        self.snapshot()
    }
    fn restore_configuration(
        &self,
        _: &PersistedConfiguration,
        snapshot: &RuntimeSnapshot,
    ) -> Result<RuntimeSnapshot, AppError> {
        self.committed_active_ids
            .lock()
            .unwrap()
            .push(snapshot.active_profile_id.clone());
        Ok(snapshot.clone())
    }
}

pub(crate) struct Fixture {
    pub(crate) service: ApplicationService,
    pub(crate) store: Arc<MemoryStore>,
    pub(crate) credentials: Arc<MemoryCredentials>,
    startup: Arc<MemoryStartup>,
    pub(crate) runtime: Arc<FakeRuntime>,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let store = Arc::new(MemoryStore::default());
        let credentials = Arc::new(MemoryCredentials::default());
        let startup = Arc::new(MemoryStartup::default());
        let runtime = Arc::new(FakeRuntime::default());
        let service = ApplicationService::new(
            Box::new(store.clone()),
            Box::new(credentials.clone()),
            Box::new(startup.clone()),
            Box::new(runtime.clone()),
        );
        Self {
            service,
            store,
            credentials,
            startup,
            runtime,
        }
    }
}

pub(crate) fn profile_input() -> ProfileInput {
    ProfileInput {
        id: None,
        name: "Primary".into(),
        protocol: ProxyProtocol::Socks5,
        host: "proxy.example.com".into(),
        port: 1080,
        authentication_enabled: false,
        enabled: true,
        credential: None,
    }
}

#[test]
fn health_checks_report_recovery_and_preserve_the_application_entrypoint() -> Result<(), AppError> {
    let mut fixture = Fixture::new();
    let initial = fixture.service.health_check()?;
    assert_eq!(initial.configuration_revision, 0);
    assert!(!initial.recovery_pending);
    assert_eq!(initial.runtime, fixture.service.runtime_snapshot());
    fixture.service.save_profile(profile_input())?;
    let health = fixture.service.health_check()?;
    assert_eq!(health.configuration_revision, 1);
    fixture.service = fixture.service.with_startup_recovery_pending(true);
    assert!(fixture.service.health_check()?.recovery_pending);
    assert!(fixture.service.select_profile(None).is_err());
    fixture.service.recover_network()?;
    assert!(!fixture.service.health_check()?.recovery_pending);
    assert!(!fixture.service.connection_history().available);
    // The facade uses the repository interface, including capability failures.
    let filter = crate::store::DiagnosticFilter {
        from_ms: None,
        until_ms: None,
        severity: None,
        search: None,
    };
    assert!(fixture.service.runtime_diagnostics(&filter, 0, 10).is_err());
    assert!(fixture
        .service
        .clear_runtime_diagnostics(&filter, true)
        .is_err());
    Ok(())
}

#[test]
fn legacy_latency_entrypoint_uses_the_registry_without_changing_selection() -> Result<(), AppError>
{
    use crate::latency_tasks::{LatencyExecutor, LatencyInput, LatencyResult, LatencyTaskRegistry};
    use std::sync::atomic::AtomicBool;
    struct Executor;
    impl LatencyExecutor for Executor {
        fn run(&self, _: LatencyInput, _: Arc<AtomicBool>) -> Result<LatencyResult, AppError> {
            Ok(LatencyResult { latency_ms: 42 })
        }
    }
    let fixture = Fixture::new();
    assert!(fixture.service.test_latency_compat("missing").is_err());
    let service = fixture
        .service
        .with_latency_tasks(LatencyTaskRegistry::new(Arc::new(Executor)));
    let profile = service.save_profile(profile_input())?;
    assert_eq!(service.test_latency_compat(&profile.id)?.latency_ms, 42);
    assert!(service.test_latency_compat("missing").is_err());
    assert_eq!(fixture.store.load()?.active_profile_id, None);
    assert_eq!(fixture.store.load_mode()?, RuntimeMode::Direct);
    Ok(())
}

fn rule(id: &str, target: &str) -> RoutingRule {
    RoutingRule {
        id: id.into(),
        name: id.into(),
        matcher: RuleMatcher::Domain,
        target: target.into(),
        port_start: None,
        port_end: None,
        action: RuleAction::Proxy,
        proxy_profile_id: None,
        enabled: true,
    }
}

#[path = "../configuration_transaction_tests.rs"]
mod transaction;

#[path = "../configuration_runtime_tests.rs"]
mod runtime_combination;
