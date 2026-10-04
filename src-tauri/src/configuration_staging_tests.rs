use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

struct JournalCredentials {
    store: Arc<MemoryStore>,
    values: Arc<MemoryCredentials>,
    fail_stage: AtomicBool,
    fail_cleanup: AtomicBool,
}

impl CredentialStore for Arc<JournalCredentials> {
    fn get(&self, reference: &str) -> Result<Option<ProxyCredential>, AppError> {
        self.values.get(reference)
    }
    fn replace(&self, reference: &str, username: &str, password: &str) -> Result<(), AppError> {
        let record = self
            .store
            .recovery_record()?
            .expect("intent must precede keyring write");
        assert!(!record.committed);
        assert!(record.intent.staged_refs.iter().any(|key| key == reference));
        assert!(!record
            .intent
            .previous
            .profiles
            .iter()
            .any(|p| p.credential_ref.as_deref() == Some(reference)));
        let json = serde_json::to_string(&record.intent).unwrap();
        assert!(!json.contains(username));
        assert!(!json.contains(password));
        self.values.replace(reference, username, password)?;
        if self.fail_stage.swap(false, Ordering::SeqCst) {
            Err(AppError::unavailable("partial keyring write"))
        } else {
            Ok(())
        }
    }
    fn delete(&self, reference: &str) -> Result<(), AppError> {
        let record = self
            .store
            .recovery_record()?
            .expect("intent must survive keyring cleanup");
        let allowed = if record.committed {
            &record.intent.retired_refs
        } else {
            &record.intent.staged_refs
        };
        assert!(allowed.iter().any(|key| key == reference));
        if self.fail_cleanup.load(Ordering::SeqCst) {
            return Err(AppError::unavailable("cleanup failed"));
        }
        self.values.delete(reference)
    }
}

struct StagingFixture {
    service: ConfigurationService,
    store: Arc<MemoryStore>,
    credentials: Arc<JournalCredentials>,
    runtime: Arc<FakeRuntime>,
}

impl StagingFixture {
    fn new() -> Self {
        let store = Arc::new(MemoryStore::default());
        let credentials = Arc::new(JournalCredentials {
            store: store.clone(),
            values: Arc::new(MemoryCredentials::default()),
            fail_stage: AtomicBool::new(false),
            fail_cleanup: AtomicBool::new(false),
        });
        let runtime = Arc::new(FakeRuntime::default());
        let service = ConfigurationService::new(
            Box::new(store.clone()),
            Box::new(credentials.clone()),
            Box::new(Arc::new(MemoryStartup::default())),
            Box::new(runtime.clone()),
        );
        Self {
            service,
            store,
            credentials,
            runtime,
        }
    }
    fn save(&self, id: Option<String>, password: &str) -> Result<ProfileView, AppError> {
        let mut input = profile_input();
        input.id = id;
        input.authentication_enabled = true;
        input.credential = Some(CredentialUpdate::Replace {
            username: "private-user".into(),
            password: password.into(),
        });
        self.service.save_profile(input)
    }
    fn reference(&self) -> String {
        self.store.load().unwrap().profiles[0]
            .credential_ref
            .clone()
            .unwrap()
    }
}

#[test]
fn secret_replacement_and_delete_follow_durable_boundary() {
    let fixture = StagingFixture::new();
    let id = fixture.save(None, "old-secret").unwrap().id;
    let original = fixture.reference();
    assert_ne!(original, id);
    fixture.save(Some(id.clone()), "new-secret").unwrap();
    let current = fixture.reference();
    assert_ne!(original, current);
    assert!(fixture.credentials.get(&original).unwrap().is_none());
    assert_eq!(
        fixture.credentials.get(&current).unwrap().unwrap().password,
        "new-secret"
    );
    assert!(fixture.store.recovery_record().unwrap().is_none());
    fixture.service.delete_profile(&id).unwrap();
    assert!(fixture.credentials.get(&current).unwrap().is_none());
    assert!(fixture.store.recovery_record().unwrap().is_none());
}

#[test]
fn partial_stage_runtime_and_database_failures_preserve_old_secret_and_reference() {
    for failure in ["stage", "runtime", "database"] {
        let fixture = StagingFixture::new();
        let id = fixture.save(None, "old-secret").unwrap().id;
        let original = fixture.reference();
        let before = fixture.store.load().unwrap();
        match failure {
            "stage" => fixture.credentials.fail_stage.store(true, Ordering::SeqCst),
            "runtime" => *fixture.runtime.reject_next.lock().unwrap() = true,
            _ => *fixture.store.fail_save.lock().unwrap() = true,
        }
        assert!(fixture.save(Some(id), "new-secret").is_err());
        assert_eq!(fixture.store.load().unwrap(), before);
        assert_eq!(fixture.reference(), original);
        assert_eq!(
            fixture
                .credentials
                .get(&original)
                .unwrap()
                .unwrap()
                .password,
            "old-secret"
        );
        assert_eq!(fixture.credentials.values.0.lock().unwrap().len(), 1);
        assert!(fixture.store.recovery_record().unwrap().is_none());
    }
}

#[test]
fn postcommit_cleanup_failure_retains_new_configuration_and_committed_marker() {
    let fixture = StagingFixture::new();
    let id = fixture.save(None, "old-secret").unwrap().id;
    let old = fixture.reference();
    fixture
        .credentials
        .fail_cleanup
        .store(true, Ordering::SeqCst);
    assert_eq!(
        fixture
            .save(Some(id.clone()), "new-secret")
            .unwrap_err()
            .code,
        "configuration_recovery"
    );
    let new = fixture.reference();
    assert_ne!(old, new);
    assert_eq!(
        fixture.service.profile_credential(&id).unwrap().password,
        "new-secret"
    );
    assert_eq!(
        fixture.credentials.get(&old).unwrap().unwrap().password,
        "old-secret"
    );
    assert!(fixture.store.recovery_record().unwrap().unwrap().committed);
    assert!(fixture.save(Some(id), "third-secret").is_err());
    assert_eq!(fixture.credentials.values.0.lock().unwrap().len(), 2);
    fixture
        .credentials
        .fail_cleanup
        .store(false, Ordering::SeqCst);
    fixture.service.recover_network().unwrap();
    assert_eq!(*fixture.runtime.recoveries.lock().unwrap(), 1);
    assert!(fixture.store.recovery_record().unwrap().is_none());
    assert!(fixture.credentials.get(&old).unwrap().is_none());
    assert_eq!(
        fixture.credentials.get(&new).unwrap().unwrap().password,
        "new-secret"
    );
    fixture.service.select_profile(None).unwrap();
}

#[test]
fn legacy_reference_upgrade_preserves_secret_and_retry_after_rejected_commit() {
    let fixture = StagingFixture::new();
    let id = fixture.save(None, "old-secret").unwrap().id;
    let versioned = fixture.reference();
    fixture
        .credentials
        .values
        .replace(&id, "private-user", "old-secret")
        .unwrap();
    fixture.credentials.values.delete(&versioned).unwrap();
    let mut previous = fixture.store.load().unwrap();
    previous.profiles[0].credential_ref = Some(id.clone());
    fixture.store.save(&previous).unwrap();
    let mut input = profile_input();
    input.id = Some(id.clone());
    input.authentication_enabled = true;
    *fixture.store.fail_save.lock().unwrap() = true;
    assert!(fixture.service.save_profile(input).is_err());
    assert_eq!(fixture.reference(), id);
    assert!(fixture.credentials.get(&id).unwrap().is_some());
    assert_eq!(fixture.credentials.values.0.lock().unwrap().len(), 1);
    *fixture.store.fail_save.lock().unwrap() = false;
    fixture.service.upgrade_legacy_credentials().unwrap();
    assert_ne!(fixture.reference(), id);
    assert!(fixture.credentials.get(&id).unwrap().is_none());
    assert_eq!(
        fixture.service.profile_credential(&id).unwrap().password,
        "old-secret"
    );
    let reference = fixture.reference();
    let revision = fixture.store.recovery_revision().unwrap();
    fixture.service.upgrade_legacy_credentials().unwrap();
    assert_eq!(fixture.reference(), reference);
    assert_eq!(fixture.store.recovery_revision().unwrap(), revision);
    let restarted = ConfigurationService::new(
        Box::new(fixture.store.clone()),
        Box::new(fixture.credentials.clone()),
        Box::new(Arc::new(MemoryStartup::default())),
        Box::new(fixture.runtime.clone()),
    );
    assert_eq!(
        restarted.list_profiles().unwrap()[0].configuration_revision,
        revision
    );
}

#[test]
fn external_startup_change_retains_original_config_and_recovery_evidence() {
    use crate::configuration_recovery::StartupEntry;
    struct StartupRace {
        store: Arc<MemoryStore>,
        value: Mutex<Option<StartupEntry>>,
    }
    impl StartupAdapter for Arc<StartupRace> {
        fn is_enabled(&self) -> Result<bool, AppError> {
            Ok(false)
        }
        fn set_enabled(&self, _: bool) -> Result<(), AppError> {
            panic!("boolean write bypass");
        }
        fn read_entry(&self) -> Result<Option<StartupEntry>, AppError> {
            Ok(self.value.lock().unwrap().clone())
        }
        fn expected_entry(&self) -> Result<StartupEntry, AppError> {
            Ok(StartupEntry {
                value_type: 1,
                bytes: vec![65],
            })
        }
        fn write_entry(&self, entry: Option<&StartupEntry>) -> Result<(), AppError> {
            let record = self
                .store
                .recovery_record()?
                .expect("startup intent precedes write");
            assert!(!record.committed);
            assert_eq!(record.intent.startup.unwrap().expected.as_ref(), entry);
            // Simulate another owner changing the value immediately after a
            // partial operation. Restoration must detect it and keep evidence.
            *self.value.lock().unwrap() = Some(StartupEntry {
                value_type: 2,
                bytes: vec![66],
            });
            Err(AppError::unavailable("partial startup write"))
        }
    }
    let fixture = StagingFixture::new();
    let startup = Arc::new(StartupRace {
        store: fixture.store.clone(),
        value: Mutex::new(None),
    });
    let service = ConfigurationService::new(
        Box::new(fixture.store.clone()),
        Box::new(fixture.credentials.clone()),
        Box::new(startup.clone()),
        Box::new(fixture.runtime.clone()),
    );
    let before = fixture.store.load().unwrap();
    let mut settings = before.settings.clone();
    settings.launch_at_login = true;
    assert_eq!(
        service.update_settings(settings).unwrap_err().code,
        "rollback_failed"
    );
    assert_eq!(
        startup.read_entry().unwrap(),
        Some(StartupEntry {
            value_type: 2,
            bytes: vec![66]
        })
    );
    assert_eq!(fixture.store.load().unwrap(), before);
    assert!(!fixture.store.recovery_record().unwrap().unwrap().committed);
    assert!(fixture
        .runtime
        .committed_active_ids
        .lock()
        .unwrap()
        .is_empty());
    assert!(service.select_profile(None).is_err());
}
