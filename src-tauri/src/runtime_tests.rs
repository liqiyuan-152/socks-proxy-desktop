use super::*;
use crate::{
    models::{ProxyProfile, ProxyProtocol},
    runtime_session::SessionLease,
};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex,
};

#[derive(Default)]
struct FakeBackend {
    fail_next: AtomicBool,
    exited: AtomicBool,
    fail_reconcile: AtomicBool,
    fail_direct: AtomicBool,
    next_pid: AtomicU32,
    restorations: AtomicU32,
    modes: Mutex<Vec<RuntimeMode>>,
}

impl RuntimeBackend for Arc<FakeBackend> {
    fn restore_network(&self) -> Result<(), AppError> {
        self.restorations.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn transition(
        &self,
        _: Option<&BackendSession>,
        _: &PersistedConfiguration,
        mode: RuntimeMode,
        revision: u64,
    ) -> Result<Option<BackendSession>, AppError> {
        self.modes.lock().unwrap().push(mode);
        if self.fail_next.swap(false, Ordering::SeqCst) {
            return Err(AppError::unavailable("内核健康检查失败"));
        }
        if mode == RuntimeMode::Direct {
            if self.fail_direct.load(Ordering::SeqCst) {
                return Err(AppError::unavailable("遗留系统代理恢复失败"));
            }
            return Ok(None);
        }
        let pid = self.next_pid.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(Some(BackendSession {
            run_id: format!("run-{pid}"),
            process_id: pid,
            configuration_revision: revision,
            system_proxy_enabled: true,
            tun_enabled: false,
        }))
    }

    fn confirm_transition(&self, _: Option<&BackendSession>) {}

    fn revert_transition(
        &self,
        _: Option<&BackendSession>,
        _: Option<&BackendSession>,
    ) -> Result<(), AppError> {
        Ok(())
    }

    fn reconcile_session(&self, _: &BackendSession) -> Result<bool, AppError> {
        if self.fail_reconcile.load(Ordering::SeqCst) {
            return Err(AppError::unavailable("系统代理恢复失败"));
        }
        Ok(!self.exited.load(Ordering::SeqCst))
    }
}
#[path = "runtime_recovery_tests.rs"]
mod recovery_tests;

fn configuration() -> PersistedConfiguration {
    let mut configuration = PersistedConfiguration::default();
    configuration.profiles.push(ProxyProfile {
        id: "stable-profile".into(),
        name: "Primary".into(),
        protocol: ProxyProtocol::Socks5,
        host: "proxy.example.com".into(),
        port: 1080,
        authentication_enabled: false,
        credential_ref: None,
        enabled: true,
    });
    configuration.active_profile_id = Some("stable-profile".into());
    configuration
}

fn manager(backend: Arc<FakeBackend>, configuration: PersistedConfiguration) -> ManagedRuntime {
    let ownership = SessionLease::acquire(&uuid::Uuid::new_v4().to_string()).unwrap();
    ManagedRuntime::from_lease(configuration, Box::new(backend), ownership).unwrap()
}
#[test]
fn startup_failure_keeps_mode_unapplied_and_clears_uptime() {
    let backend = Arc::new(FakeBackend::default());
    backend.fail_next.store(true, Ordering::SeqCst);
    let runtime = manager(backend, configuration());
    assert!(runtime.request_mode(RuntimeMode::Global).is_err());
    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.phase, RuntimePhase::Failed);
    assert_eq!(snapshot.applied_mode, None);
    assert_eq!(snapshot.runtime_uptime_ms, None);
    assert_eq!(snapshot.revision, 0);
}

#[test]
fn unexpected_exit_invalidates_applied_mode_and_uptime() {
    let backend = Arc::new(FakeBackend::default());
    let runtime = manager(backend.clone(), configuration());
    runtime.request_mode(RuntimeMode::Global).unwrap();
    backend.exited.store(true, Ordering::SeqCst);
    let failed = runtime.snapshot();
    assert_eq!(failed.phase, RuntimePhase::Failed);
    assert_eq!(failed.applied_mode, None);
    assert_eq!(failed.runtime_uptime_ms, None);
    assert!(failed.last_error.unwrap().contains("意外退出"));
}

#[test]
fn hot_switch_failure_keeps_previous_mode_and_uptime() {
    let backend = Arc::new(FakeBackend::default());
    let runtime = manager(backend.clone(), configuration());
    let running = runtime.request_mode(RuntimeMode::Global).unwrap();
    assert_eq!(running.phase, RuntimePhase::Running);
    assert_eq!(running.coverage, TrafficCoverage::SystemProxyApps);
    assert!(!running.tun_enabled);
    assert!(running.runtime_uptime_ms.is_some());
    backend.fail_next.store(true, Ordering::SeqCst);
    assert!(runtime.request_mode(RuntimeMode::Rules).is_err());
    let failed = runtime.snapshot();
    assert_eq!(failed.applied_mode, Some(RuntimeMode::Global));
    assert_eq!(failed.desired_mode, RuntimeMode::Rules);
    assert_eq!(failed.revision, running.revision);
    assert!(failed.runtime_uptime_ms.is_some());
    let switched = runtime.request_mode(RuntimeMode::Rules).unwrap();
    assert_eq!(switched.applied_mode, Some(RuntimeMode::Rules));
    assert_eq!(switched.revision, running.revision + 1);
    assert!(switched.runtime_uptime_ms.unwrap() >= running.runtime_uptime_ms.unwrap());
}

#[test]
fn direct_and_stop_clear_running_session_and_uptime() {
    let backend = Arc::new(FakeBackend::default());
    let runtime = manager(backend.clone(), configuration());
    runtime.request_mode(RuntimeMode::Rules).unwrap();
    let direct = runtime.request_mode(RuntimeMode::Direct).unwrap();
    assert_eq!(direct.phase, RuntimePhase::Stopped);
    assert_eq!(direct.applied_mode, Some(RuntimeMode::Direct));
    assert_eq!(direct.runtime_uptime_ms, None);
    assert!(!direct.system_proxy_enabled);
    assert_eq!(direct.coverage, TrafficCoverage::None);
    runtime.request_mode(RuntimeMode::Global).unwrap();
    let stopped = runtime.stop().unwrap();
    assert_eq!(stopped.applied_mode, None);
    assert_eq!(stopped.runtime_uptime_ms, None);
    assert_eq!(
        backend.modes.lock().unwrap().last(),
        Some(&RuntimeMode::Direct)
    );
}

#[test]
fn deleting_active_profile_and_restoring_snapshot_recovers_mode() {
    let backend = Arc::new(FakeBackend::default());
    let config = configuration();
    let runtime = manager(backend, config.clone());
    let before = runtime.request_mode(RuntimeMode::Global).unwrap();
    let mut candidate = config.clone();
    candidate.profiles.clear();
    candidate.active_profile_id = None;
    let staged = runtime.apply_configuration(&config, &candidate).unwrap();
    assert_eq!(staged.applied_mode, Some(RuntimeMode::Global));
    assert_eq!(staged.active_profile_id, Some("stable-profile".into()));
    assert_eq!(staged.revision, before.revision);
    assert_eq!(staged.phase, RuntimePhase::Recovering);
    let restored = runtime.restore_configuration(&config, &before).unwrap();
    assert_eq!(restored.applied_mode, Some(RuntimeMode::Global));
    assert_eq!(restored.active_profile_id, Some("stable-profile".into()));
    assert_eq!(restored.revision, before.revision);
    assert!(restored.runtime_uptime_ms.is_some());

    runtime.apply_configuration(&config, &candidate).unwrap();
    let direct = runtime.confirm_configuration();
    assert_eq!(direct.applied_mode, Some(RuntimeMode::Direct));
    assert_eq!(direct.active_profile_id, None);
    assert_eq!(direct.revision, before.revision + 1);
}

#[test]
fn candidate_failure_keeps_previous_configuration_and_mode() {
    let backend = Arc::new(FakeBackend::default());
    let config = configuration();
    let runtime = manager(backend.clone(), config.clone());
    runtime.request_mode(RuntimeMode::Rules).unwrap();
    let mut candidate = config.clone();
    candidate.rules.push(crate::models::RoutingRule {
        id: "rule".into(),
        name: "Rule".into(),
        matcher: crate::models::RuleMatcher::Domain,
        target: "example.com".into(),
        port_start: None,
        port_end: None,
        action: crate::models::RuleAction::Direct,
        enabled: true,
    });
    backend.fail_next.store(true, Ordering::SeqCst);
    assert!(runtime.apply_configuration(&config, &candidate).is_err());
    assert_eq!(runtime.snapshot().applied_mode, Some(RuntimeMode::Rules));
    assert_eq!(runtime.snapshot().revision, 1);
    assert!(runtime.apply_configuration(&config, &candidate).is_ok());
}

#[test]
fn persistence_failure_restores_runtime_revision_and_previous_rules() {
    use crate::{
        configuration_service::ConfigurationService,
        credentials::{CredentialStore, ProxyCredential},
        startup::StartupAdapter,
        store::ConfigurationStore,
    };

    struct Store {
        configuration: Mutex<PersistedConfiguration>,
        reject: AtomicBool,
    }
    impl ConfigurationStore for Arc<Store> {
        fn load(&self) -> Result<PersistedConfiguration, AppError> {
            Ok(self.configuration.lock().unwrap().clone())
        }
        fn save(&self, value: &PersistedConfiguration) -> Result<(), AppError> {
            if self.reject.load(Ordering::SeqCst) {
                Err(AppError::storage("测试持久化失败"))
            } else {
                *self.configuration.lock().unwrap() = value.clone();
                Ok(())
            }
        }
    }
    struct Credentials;
    impl CredentialStore for Credentials {
        fn get(&self, _: &str) -> Result<Option<ProxyCredential>, AppError> {
            Ok(None)
        }
        fn replace(&self, _: &str, _: &str, _: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn delete(&self, _: &str) -> Result<(), AppError> {
            Ok(())
        }
    }
    struct Startup;
    impl StartupAdapter for Startup {
        fn is_enabled(&self) -> Result<bool, AppError> {
            Ok(false)
        }
        fn set_enabled(&self, _: bool) -> Result<(), AppError> {
            Ok(())
        }
    }

    let config = configuration();
    let store = Arc::new(Store {
        configuration: Mutex::new(config.clone()),
        reject: AtomicBool::new(false),
    });
    let runtime = manager(Arc::new(FakeBackend::default()), config);
    let service = ConfigurationService::new(
        Box::new(store.clone()),
        Box::new(Credentials),
        Box::new(Startup),
        Box::new(runtime),
    );
    let before = service.request_mode(RuntimeMode::Rules).unwrap();
    let rule = crate::models::RoutingRule {
        id: "new-rule".into(),
        name: "New".into(),
        matcher: crate::models::RuleMatcher::Domain,
        target: "example.com".into(),
        port_start: None,
        port_end: None,
        action: crate::models::RuleAction::Proxy,
        enabled: true,
    };
    store.reject.store(true, Ordering::SeqCst);
    assert_eq!(
        service.replace_rules(vec![rule]).unwrap_err().code,
        "storage_error"
    );
    let restored = service.runtime_snapshot();
    assert_eq!(restored.revision, before.revision);
    assert_eq!(restored.applied_mode, Some(RuntimeMode::Rules));
    assert_eq!(restored.active_profile_id, before.active_profile_id);
    assert!(restored.runtime_uptime_ms.is_some());
    assert!(store.load().unwrap().rules.is_empty());
}

#[test]
fn in_flight_switch_exposes_stage_without_prematurely_committing_mode() {
    use std::sync::mpsc::{channel, Receiver, Sender};
    struct GatedBackend {
        entered: Sender<RuntimeMode>,
        resume: Mutex<Receiver<()>>,
    }
    impl RuntimeBackend for GatedBackend {
        fn transition(
            &self,
            _: Option<&BackendSession>,
            _: &PersistedConfiguration,
            mode: RuntimeMode,
            revision: u64,
        ) -> Result<Option<BackendSession>, AppError> {
            self.entered.send(mode).unwrap();
            self.resume.lock().unwrap().recv().unwrap();
            Ok(Some(BackendSession {
                run_id: format!("run-{revision}"),
                process_id: 42,
                configuration_revision: revision,
                system_proxy_enabled: true,
                tun_enabled: false,
            }))
        }

        fn confirm_transition(&self, _: Option<&BackendSession>) {}

        fn revert_transition(
            &self,
            _: Option<&BackendSession>,
            _: Option<&BackendSession>,
        ) -> Result<(), AppError> {
            Ok(())
        }

        fn reconcile_session(&self, _: &BackendSession) -> Result<bool, AppError> {
            Ok(true)
        }
    }
    let (entered_tx, entered_rx) = channel();
    let (resume_tx, resume_rx) = channel();
    let lease = SessionLease::acquire(&uuid::Uuid::new_v4().to_string()).unwrap();
    let runtime = Arc::new(
        ManagedRuntime::from_lease(
            configuration(),
            Box::new(GatedBackend {
                entered: entered_tx,
                resume: Mutex::new(resume_rx),
            }),
            lease,
        )
        .unwrap(),
    );
    let starting = Arc::clone(&runtime);
    let first = std::thread::spawn(move || starting.request_mode(RuntimeMode::Global));
    assert_eq!(
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap(),
        RuntimeMode::Global
    );
    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.phase, RuntimePhase::Starting);
    assert_eq!(snapshot.applied_mode, None);
    assert_eq!(snapshot.runtime_uptime_ms, None);
    resume_tx.send(()).unwrap();
    let committed = first.join().unwrap().unwrap();
    assert_eq!(committed.applied_mode, Some(RuntimeMode::Global));

    let switching = Arc::clone(&runtime);
    let second = std::thread::spawn(move || switching.request_mode(RuntimeMode::Rules));
    assert_eq!(
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap(),
        RuntimeMode::Rules
    );
    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.phase, RuntimePhase::Switching);
    assert_eq!(snapshot.applied_mode, Some(RuntimeMode::Global));
    assert_eq!(snapshot.revision, committed.revision);
    assert!(snapshot.runtime_uptime_ms.is_some());
    resume_tx.send(()).unwrap();
    let switched = second.join().unwrap().unwrap();
    assert_eq!(switched.applied_mode, Some(RuntimeMode::Rules));
    assert!(switched.runtime_uptime_ms.unwrap() >= committed.runtime_uptime_ms.unwrap());
}
