use super::*;
use crate::{
    credentials::ProxyCredential,
    models::{RetentionPolicy, RuleAction, RuleMatcher, RuntimeMode},
    runtime::{RuntimePhase, RuntimeSnapshot},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

struct MemoryStore {
    value: Mutex<PersistedConfiguration>,
    mode: Mutex<RuntimeMode>,
    fail_save: Mutex<bool>,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self {
            value: Mutex::new(PersistedConfiguration::default()),
            mode: Mutex::new(RuntimeMode::Direct),
            fail_save: Mutex::new(false),
        }
    }
}

impl ConfigurationStore for Arc<MemoryStore> {
    fn load(&self) -> Result<PersistedConfiguration, AppError> {
        Ok(self.value.lock().unwrap().clone())
    }

    fn save(&self, candidate: &PersistedConfiguration) -> Result<(), AppError> {
        if *self.fail_save.lock().unwrap() {
            return Err(AppError::storage("测试写入失败"));
        }
        *self.value.lock().unwrap() = candidate.clone();
        Ok(())
    }
    fn load_mode(&self) -> Result<RuntimeMode, AppError> {
        Ok(*self.mode.lock().unwrap())
    }
    fn save_mode(&self, mode: RuntimeMode) -> Result<(), AppError> {
        if *self.fail_save.lock().unwrap() {
            return Err(AppError::storage("测试写入失败"));
        }
        *self.mode.lock().unwrap() = mode;
        Ok(())
    }
}

#[derive(Default)]
struct MemoryCredentials(Mutex<HashMap<String, ProxyCredential>>);

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
struct MemoryStartup(Mutex<bool>);

impl StartupAdapter for Arc<MemoryStartup> {
    fn is_enabled(&self) -> Result<bool, AppError> {
        Ok(*self.0.lock().unwrap())
    }
    fn set_enabled(&self, enabled: bool) -> Result<(), AppError> {
        *self.0.lock().unwrap() = enabled;
        Ok(())
    }
}

#[derive(Default)]
struct FakeRuntime {
    reject_next: Mutex<bool>,
    committed_active_ids: Mutex<Vec<Option<String>>>,
}

impl RuntimeCoordinator for Arc<FakeRuntime> {
    fn snapshot(&self) -> RuntimeSnapshot {
        RuntimeSnapshot {
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
            last_error: None,
        }
    }
    fn request_mode(&self, _: RuntimeMode) -> Result<RuntimeSnapshot, AppError> {
        Ok(self.snapshot())
    }
    fn stop(&self) -> Result<RuntimeSnapshot, AppError> {
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

struct Fixture {
    service: ConfigurationService,
    store: Arc<MemoryStore>,
    credentials: Arc<MemoryCredentials>,
    startup: Arc<MemoryStartup>,
    runtime: Arc<FakeRuntime>,
}

impl Fixture {
    fn new() -> Self {
        let store = Arc::new(MemoryStore::default());
        let credentials = Arc::new(MemoryCredentials::default());
        let startup = Arc::new(MemoryStartup::default());
        let runtime = Arc::new(FakeRuntime::default());
        let service = ConfigurationService::new(
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

fn profile_input() -> ProfileInput {
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
fn china_preset_requires_default_and_valid_bundled_rules_before_commit() {
    let mut fixture = Fixture::new();
    assert!(!fixture.service.china_direct_status().unwrap().available);
    assert!(fixture.service.set_china_direct_enabled(true).is_err());
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/china-rules");
    fixture.service = fixture.service.with_china_rule_root(root);
    assert!(fixture.service.china_direct_status().unwrap().available);
    assert_eq!(
        fixture
            .service
            .set_china_direct_enabled(true)
            .unwrap_err()
            .fields[0]
            .field,
        "default_profile_id"
    );
    let profile = fixture.service.save_profile(profile_input()).unwrap();
    fixture.service.select_profile(Some(profile.id)).unwrap();
    let status = fixture.service.set_china_direct_enabled(true).unwrap();
    assert!(status.enabled && status.available && status.data_date.is_some());
    assert!(fixture.store.load().unwrap().china_direct_enabled);
}

#[test]
fn invalid_china_rules_reject_revision_without_changing_stored_configuration() {
    let mut fixture = Fixture::new();
    let profile = fixture.service.save_profile(profile_input()).unwrap();
    fixture.service.select_profile(Some(profile.id)).unwrap();
    let missing = tempfile::tempdir().unwrap();
    fixture.service = fixture.service.with_china_rule_root(missing.path().into());
    let error = fixture.service.set_china_direct_enabled(true).unwrap_err();
    assert!(error.message.contains("规则集"));
    assert!(!fixture.store.load().unwrap().china_direct_enabled);
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

#[test]
fn credentials_are_read_only_for_existing_authenticated_profiles() {
    let fixture = Fixture::new();
    let mut input = profile_input();
    input.authentication_enabled = true;
    input.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "secret".into(),
    });
    let created = fixture.service.save_profile(input).unwrap();
    let credential = fixture.service.profile_credential(&created.id).unwrap();
    assert_eq!(credential.username, "alice");
    assert_eq!(credential.password, "secret");
    assert_eq!(fixture.service.list_profiles().unwrap()[0].name, "Primary");
    assert_eq!(
        fixture
            .service
            .profile_credential("missing")
            .err()
            .unwrap()
            .code,
        "not_found"
    );

    let mut plain = profile_input();
    plain.name = "Plain".into();
    let plain = fixture.service.save_profile(plain).unwrap();
    assert_eq!(
        fixture
            .service
            .profile_credential(&plain.id)
            .err()
            .unwrap()
            .code,
        "unavailable"
    );
    fixture.credentials.delete(&created.id).unwrap();
    assert_eq!(
        fixture
            .service
            .profile_credential(&created.id)
            .err()
            .unwrap()
            .code,
        "unavailable"
    );
}

#[test]
fn validates_profile_inputs_before_writing_and_returns_field_errors() {
    let fixture = Fixture::new();
    let created = fixture.service.save_profile(profile_input()).unwrap();
    assert!(!created.id.is_empty());
    let mut duplicate = profile_input();
    duplicate.name = "primary".into();
    assert_eq!(
        fixture.service.save_profile(duplicate).unwrap_err().fields[0].field,
        "profiles[1].name"
    );
    let mut invalid = profile_input();
    invalid.name = "Secondary".into();
    invalid.port = 0;
    assert_eq!(
        fixture.service.save_profile(invalid).unwrap_err().fields[0].field,
        "profiles[1].port"
    );
    assert_eq!(fixture.service.list_profiles().unwrap().len(), 1);
}

#[test]
fn deleting_active_profile_waits_for_runtime_and_rolls_back_secret_on_failure() {
    let fixture = Fixture::new();
    let mut input = profile_input();
    input.authentication_enabled = true;
    input.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "secret-value".into(),
    });
    let created = fixture.service.save_profile(input).unwrap();
    fixture
        .service
        .select_profile(Some(created.id.clone()))
        .unwrap();
    assert_eq!(
        fixture
            .service
            .delete_profile(&created.id)
            .unwrap_err()
            .fields[0]
            .field,
        "default_profile_id"
    );
    fixture.service.select_profile(None).unwrap();
    *fixture.runtime.reject_next.lock().unwrap() = true;
    assert!(fixture.service.delete_profile(&created.id).is_err());
    assert_eq!(fixture.store.load().unwrap().active_profile_id, None);
    assert_eq!(
        fixture
            .credentials
            .get(&created.id)
            .unwrap()
            .unwrap()
            .password,
        "secret-value"
    );
    fixture.service.delete_profile(&created.id).unwrap();
    assert_eq!(fixture.store.load().unwrap().active_profile_id, None);
    assert!(fixture.credentials.get(&created.id).unwrap().is_none());
    assert_eq!(
        fixture.runtime.committed_active_ids.lock().unwrap().last(),
        Some(&None)
    );
}

#[path = "configuration_transaction_tests.rs"]
mod transaction;
