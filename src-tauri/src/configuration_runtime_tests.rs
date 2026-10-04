use super::*;
use crate::{
    runtime::{BackendSession, ManagedRuntime, RuntimeBackend},
    runtime_session::SessionLease,
};
use std::sync::atomic::{AtomicBool, Ordering};

#[path = "configuration_plan_tests.rs"]
mod plan_tests;

struct RecordedRun {
    configuration: PersistedConfiguration,
    credential: (String, String),
    revision: u64,
}

struct RecordingBackend {
    credentials: Arc<MemoryCredentials>,
    runs: Mutex<HashMap<String, RecordedRun>>,
    fail_next: AtomicBool,
}

impl RuntimeBackend for Arc<RecordingBackend> {
    fn transition(
        &self,
        _: Option<&BackendSession>,
        configuration: &PersistedConfiguration,
        mode: RuntimeMode,
        revision: u64,
    ) -> Result<Option<BackendSession>, AppError> {
        if self.fail_next.swap(false, Ordering::SeqCst) {
            return Err(AppError::unavailable("candidate startup failed"));
        }
        if mode == RuntimeMode::Direct {
            return Ok(None);
        }
        let id = configuration.active_profile_id.as_ref().unwrap();
        let profile = configuration.profiles.iter().find(|p| &p.id == id).unwrap();
        let secret =
            crate::credentials::read_profile_credential(&self.credentials, profile)?.unwrap();
        let run_id = uuid::Uuid::new_v4().to_string();
        self.runs.lock().unwrap().insert(
            run_id.clone(),
            RecordedRun {
                configuration: configuration.clone(),
                credential: (secret.username, secret.password),
                revision,
            },
        );
        Ok(Some(BackendSession {
            run_id,
            process_id: revision as u32,
            configuration_revision: revision,
            system_proxy_enabled: true,
            tun_enabled: false,
        }))
    }
    fn confirm_transition(&self, previous: Option<&BackendSession>) {
        if let Some(previous) = previous {
            self.runs.lock().unwrap().remove(&previous.run_id);
        }
    }
    fn revert_transition(
        &self,
        candidate: Option<&BackendSession>,
        _: Option<&BackendSession>,
    ) -> Result<(), AppError> {
        if let Some(candidate) = candidate {
            self.runs.lock().unwrap().remove(&candidate.run_id);
        }
        Ok(())
    }
    fn reconcile_session(&self, session: &BackendSession) -> Result<bool, AppError> {
        Ok(self.runs.lock().unwrap().contains_key(&session.run_id))
    }
}

struct RuntimeFixture {
    service: ApplicationService,
    store: Arc<MemoryStore>,
    backend: Arc<RecordingBackend>,
    id: String,
}

impl RuntimeFixture {
    fn new() -> Self {
        let store = Arc::new(MemoryStore::default());
        let credentials = Arc::new(MemoryCredentials::default());
        let backend = Arc::new(RecordingBackend {
            credentials: credentials.clone(),
            runs: Mutex::new(HashMap::new()),
            fail_next: AtomicBool::new(false),
        });
        let lease = SessionLease::acquire(&uuid::Uuid::new_v4().to_string()).unwrap();
        let runtime = ManagedRuntime::from_lease(
            store.load().unwrap(),
            Box::new(backend.clone()),
            lease,
            RuntimeMode::Direct,
        )
        .unwrap();
        let service = ApplicationService::new(
            Box::new(store.clone()),
            Box::new(credentials.clone()),
            Box::new(Arc::new(MemoryStartup::default())),
            Box::new(runtime),
        );
        let mut input = profile_input();
        input.authentication_enabled = true;
        input.credential = Some(Self::update("alice", "old-password"));
        let id = service.save_profile(input).unwrap().id;
        service.select_profile(Some(id.clone())).unwrap();
        service.request_mode(RuntimeMode::Global).unwrap();
        Self {
            service,
            store,
            backend,
            id,
        }
    }

    fn update(username: &str, password: &str) -> CredentialUpdate {
        CredentialUpdate::Replace {
            username: username.into(),
            password: password.into(),
        }
    }

    fn save(&self, username: &str, password: &str) -> Result<ProfileView, AppError> {
        let mut input = profile_input();
        input.id = Some(self.id.clone());
        input.authentication_enabled = true;
        input.credential = Some(Self::update(username, password));
        self.service.save_profile(input)
    }

    fn assert_live(&self, username: &str, password: &str, revision: u64) {
        let runs = self.backend.runs.lock().unwrap();
        assert_eq!(runs.len(), 1);
        let run = runs.values().next().unwrap();
        assert_eq!(run.credential, (username.into(), password.into()));
        assert_eq!(run.revision, revision);
        assert_eq!(run.configuration, self.store.load().unwrap());
    }
}

#[test]
fn credential_only_updates_reach_real_coordinator_and_backend() {
    for (username, password) in [("alice", "new-password"), ("bob", "old-password")] {
        let fixture = RuntimeFixture::new();
        let before = fixture.service.runtime_snapshot();
        let configuration = fixture.service.export().unwrap();
        fixture.save(username, password).unwrap();
        assert_eq!(fixture.service.export().unwrap(), configuration);
        let after = fixture.service.runtime_snapshot();
        assert_eq!(after.revision, before.revision + 1);
        fixture.assert_live(username, password, after.runtime_plan_revision);
    }
}

#[test]
fn identical_import_with_replaced_credentials_reaches_backend() {
    let fixture = RuntimeFixture::new();
    let before = fixture.service.runtime_snapshot();
    let configuration = fixture.service.export().unwrap();
    fixture
        .service
        .import(
            &fixture.service.export().unwrap(),
            HashMap::from([(
                fixture.id.clone(),
                RuntimeFixture::update("bob", "import-password"),
            )]),
        )
        .unwrap();
    assert_eq!(fixture.service.export().unwrap(), configuration);
    let after = fixture.service.runtime_snapshot();
    assert_eq!(after.revision, before.revision + 1);
    fixture.assert_live("bob", "import-password", after.runtime_plan_revision);
}

#[test]
fn startup_and_storage_failure_retain_old_secret_and_actual_session() {
    for storage_failure in [false, true] {
        let fixture = RuntimeFixture::new();
        let before = fixture.service.runtime_snapshot();
        let previous_run = fixture
            .backend
            .runs
            .lock()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .clone();
        *fixture.store.fail_save.lock().unwrap() = storage_failure;
        fixture
            .backend
            .fail_next
            .store(!storage_failure, Ordering::SeqCst);
        assert!(fixture.save("bob", "new-password").is_err());
        let secret = fixture.service.profile_credential(&fixture.id).unwrap();
        assert_eq!(secret.username, "alice");
        assert_eq!(secret.password, "old-password");
        assert_eq!(fixture.service.runtime_snapshot().revision, before.revision);
        assert!(fixture
            .backend
            .runs
            .lock()
            .unwrap()
            .contains_key(&previous_run));
        fixture.assert_live("alice", "old-password", before.runtime_plan_revision);
    }
}
