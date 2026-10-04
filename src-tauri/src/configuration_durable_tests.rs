use super::*;
use crate::configuration_recovery::RecoveryIntent;

#[test]
fn pending_durable_intent_blocks_all_mutations_before_external_effects() {
    let store = Arc::new(crate::store::SqliteConfigurationStore::open_in_memory().unwrap());
    let credentials = Arc::new(MemoryCredentials::default());
    let startup = Arc::new(MemoryStartup::default());
    let runtime = Arc::new(FakeRuntime::default());
    let service = ConfigurationService::new(
        Box::new(store.clone()),
        Box::new(credentials.clone()),
        Box::new(startup.clone()),
        Box::new(runtime.clone()),
    );
    let previous = store.load().unwrap();
    let intent = RecoveryIntent {
        transaction_id: Uuid::new_v4().to_string(),
        previous_revision: 0,
        next_revision: 1,
        previous: previous.clone(),
        candidate: previous.clone(),
        staged_refs: vec![],
        retired_refs: vec![],
        startup: None,
    };
    store.begin_recovery(&intent).unwrap();
    let mut input = profile_input();
    input.authentication_enabled = true;
    input.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "secret".into(),
    });
    assert_eq!(
        service.save_profile(input).unwrap_err().code,
        "configuration_recovery"
    );
    assert!(service
        .import(&service.export().unwrap(), HashMap::new())
        .is_err());
    assert!(service.delete_profile("missing").is_err());
    assert!(service.select_profile(None).is_err());
    assert!(service.replace_rules(vec![]).is_err());
    assert!(service.set_china_direct_enabled(false).is_err());
    let mut settings = previous.settings.clone();
    settings.launch_at_login = true;
    assert!(service.update_settings(settings).is_err());
    assert!(service.request_mode(RuntimeMode::Global).is_err());
    assert!(service.stop_runtime().is_err());
    assert!(credentials.0.lock().unwrap().is_empty());
    assert!(!*startup.0.lock().unwrap());
    assert!(runtime.committed_active_ids.lock().unwrap().is_empty());
    // Read-only access remains available to diagnose/recover the transaction.
    assert!(service.list_profiles().unwrap().is_empty());
    assert!(service.list_rules().unwrap().is_empty());
    assert!(service.settings().is_ok());
    assert_eq!(store.load().unwrap(), previous);
    assert_eq!(store.recovery_revision().unwrap(), 0);
}

#[test]
fn pure_configuration_commit_uses_atomic_revision_and_clears_durable_intent() {
    let store = Arc::new(crate::store::SqliteConfigurationStore::open_in_memory().unwrap());
    let runtime = Arc::new(FakeRuntime::default());
    let service = ConfigurationService::new(
        Box::new(store.clone()),
        Box::new(Arc::new(MemoryCredentials::default())),
        Box::new(Arc::new(MemoryStartup::default())),
        Box::new(runtime.clone()),
    );
    service.select_profile(None).unwrap();
    assert_eq!(store.recovery_revision().unwrap(), 1);
    assert!(store.recovery_record().unwrap().is_none());
    *runtime.reject_next.lock().unwrap() = true;
    assert!(service.replace_rules(vec![]).is_err());
    assert_eq!(store.recovery_revision().unwrap(), 1);
    assert!(store.recovery_record().unwrap().is_none());
    service.replace_rules(vec![]).unwrap();
    assert_eq!(store.recovery_revision().unwrap(), 2);
    assert!(store.recovery_record().unwrap().is_none());
}

#[test]
fn credential_read_uses_saved_reference_and_export_hides_it() {
    let store = Arc::new(crate::store::SqliteConfigurationStore::open_in_memory().unwrap());
    let credentials = Arc::new(MemoryCredentials::default());
    let service = ConfigurationService::new(
        Box::new(store.clone()),
        Box::new(credentials.clone()),
        Box::new(Arc::new(MemoryStartup::default())),
        Box::new(Arc::new(FakeRuntime::default())),
    );
    let mut configuration = store.load().unwrap();
    configuration.profiles.push(ProxyProfile {
        id: "profile".into(),
        name: "Primary".into(),
        protocol: ProxyProtocol::Socks5,
        host: "proxy.example.com".into(),
        port: 1080,
        authentication_enabled: true,
        credential_ref: Some("credential-v1-current".into()),
        enabled: true,
    });
    store.save(&configuration).unwrap();
    credentials
        .replace("profile", "retired-user", "retired-secret")
        .unwrap();
    assert!(service.profile_credential("profile").is_err());
    credentials
        .replace("credential-v1-current", "current-user", "current-secret")
        .unwrap();
    assert_eq!(
        service.profile_credential("profile").unwrap().password,
        "current-secret"
    );
    let exported = service.export().unwrap();
    for forbidden in [
        "credential-v1-current",
        "current-user",
        "current-secret",
        "retired-secret",
    ] {
        assert!(!exported.contains(forbidden));
    }
}

#[test]
fn predecessor_recovery_failure_blocks_mutations_even_without_config_journal() {
    let mut fixture = Fixture::new();
    fixture.service = fixture.service.with_startup_recovery_pending(true);
    assert!(fixture.store.recovery_record().unwrap().is_none());
    assert!(fixture.service.select_profile(None).is_err());
    assert!(fixture.service.request_mode(RuntimeMode::Global).is_err());
    assert!(fixture.service.list_profiles().is_ok());
    fixture.service.recover_network().unwrap();
    assert_eq!(*fixture.runtime.recoveries.lock().unwrap(), 1);
    fixture.service.select_profile(None).unwrap();
}

#[test]
fn latency_snapshot_reads_exact_secret_and_revision_then_commit_invalidates_task() {
    use crate::latency_tasks::{
        LatencyExecutor, LatencyInput, LatencyResult, LatencyTaskRegistry, LatencyTaskState,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Executor;
    impl LatencyExecutor for Executor {
        fn run(
            &self,
            input: LatencyInput,
            cancelled: Arc<AtomicBool>,
        ) -> Result<LatencyResult, AppError> {
            assert_eq!(input.configuration_revision, 1);
            assert_eq!(
                input.configuration.profiles[0].credential_ref.as_deref(),
                Some("credential-v1-current")
            );
            assert_eq!(input.credential.unwrap().password, "current-secret");
            while !cancelled.load(Ordering::Acquire) {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Ok(LatencyResult { latency_ms: 999 })
        }
    }
    let fixture = Fixture::new();
    let mut configuration = fixture.store.load().unwrap();
    configuration.profiles.push(ProxyProfile {
        id: "profile".into(),
        name: "Primary".into(),
        protocol: ProxyProtocol::Socks5,
        host: "proxy.example.com".into(),
        port: 1080,
        enabled: true,
        authentication_enabled: true,
        credential_ref: Some("credential-v1-current".into()),
    });
    fixture.store.save(&configuration).unwrap();
    fixture
        .credentials
        .replace("credential-v1-current", "current-user", "current-secret")
        .unwrap();
    let service = fixture
        .service
        .with_latency_tasks(LatencyTaskRegistry::new(Arc::new(Executor)));
    let subscription = service.start_latency_task("profile").unwrap();
    assert_eq!(subscription.task.configuration_revision, 1);
    service.select_profile(None).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let snapshot = service
            .latency_task_snapshot(&subscription.subscription_id)
            .unwrap();
        if snapshot.state == LatencyTaskState::Cancelled {
            assert!(snapshot.result.is_none());
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    service
        .release_latency_task(&subscription.subscription_id)
        .unwrap();
}

#[test]
fn route_prediction_returns_normalized_input_and_durable_committed_revision() {
    let fixture = Fixture::new();
    let result = fixture.service.test_route(" EXAMPLE.COM. ", 8443).unwrap();
    assert_eq!(result.target, "example.com");
    assert_eq!(result.port, 8443);
    assert_eq!(result.configuration_revision, 0);
    fixture.service.replace_rules(vec![]).unwrap();
    let result = fixture.service.test_route("[::1]", 80).unwrap();
    assert_eq!(result.target, "::1");
    assert_eq!(result.configuration_revision, 1);
    *fixture.runtime.reject_next.lock().unwrap() = true;
    assert!(fixture.service.replace_rules(vec![]).is_err());
    assert_eq!(
        fixture
            .service
            .test_route("example.com", 443)
            .unwrap()
            .configuration_revision,
        1
    );
}
