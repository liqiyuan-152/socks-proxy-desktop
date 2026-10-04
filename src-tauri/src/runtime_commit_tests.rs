use super::*;

#[test]
fn persistence_failure_restores_runtime_revision_and_previous_rules() {
    use crate::{
        credentials::{CredentialStore, ProxyCredential},
        services::ApplicationService,
        startup::StartupAdapter,
        store::ConfigurationStore,
    };

    struct Store {
        configuration: crate::store::SqliteConfigurationStore,
        reject: AtomicBool,
    }
    impl ConfigurationStore for Arc<Store> {
        fn load(&self) -> Result<PersistedConfiguration, AppError> {
            self.configuration.load()
        }
        fn save(&self, value: &PersistedConfiguration) -> Result<(), AppError> {
            if self.reject.load(Ordering::SeqCst) {
                Err(AppError::storage("测试持久化失败"))
            } else {
                self.configuration.save(value)
            }
        }
        fn load_mode(&self) -> Result<RuntimeMode, AppError> {
            Ok(RuntimeMode::Direct)
        }
        fn save_mode(&self, _: RuntimeMode) -> Result<(), AppError> {
            Ok(())
        }
        fn recovery_revision(&self) -> Result<u64, AppError> {
            self.configuration.recovery_revision()
        }
        fn recovery_record(
            &self,
        ) -> Result<Option<crate::configuration_recovery::RecoveryRecord>, AppError> {
            self.configuration.recovery_record()
        }
        fn begin_recovery(
            &self,
            intent: &crate::configuration_recovery::RecoveryIntent,
        ) -> Result<(), AppError> {
            self.configuration.begin_recovery(intent)
        }
        fn commit_recovery(&self, id: &str) -> Result<(), AppError> {
            if self.reject.load(Ordering::SeqCst) {
                return Err(AppError::storage("测试持久化失败"));
            }
            self.configuration.commit_recovery(id)
        }
        fn clear_recovery(&self, id: &str) -> Result<(), AppError> {
            self.configuration.clear_recovery(id)
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
    let configuration_store = crate::store::SqliteConfigurationStore::open_in_memory().unwrap();
    configuration_store.save(&config).unwrap();
    let store = Arc::new(Store {
        configuration: configuration_store,
        reject: AtomicBool::new(false),
    });
    let runtime = manager(Arc::new(FakeBackend::default()), config);
    let service = ApplicationService::new(
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
        proxy_profile_id: Some("stable-profile".into()),
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
            RuntimeMode::Direct,
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
