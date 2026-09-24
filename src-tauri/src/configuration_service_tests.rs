use super::*;
use crate::{
    models::{RetentionPolicy, RuleAction, RuleMatcher, RuntimeMode},
    runtime::{RuntimePhase, RuntimeSnapshot},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct MemoryStore {
    value: Mutex<PersistedConfiguration>,
    fail_save: Mutex<bool>,
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

fn rule(id: &str, target: &str) -> RoutingRule {
    RoutingRule {
        id: id.into(),
        name: id.into(),
        matcher: RuleMatcher::Domain,
        target: target.into(),
        port_start: None,
        port_end: None,
        action: RuleAction::Proxy,
        enabled: true,
    }
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
    *fixture.runtime.reject_next.lock().unwrap() = true;
    assert!(fixture.service.delete_profile(&created.id).is_err());
    assert_eq!(
        fixture.store.load().unwrap().active_profile_id.as_deref(),
        Some(created.id.as_str())
    );
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

#[test]
fn storage_failure_restores_old_credentials_and_runtime_revision() {
    let fixture = Fixture::new();
    let mut input = profile_input();
    input.authentication_enabled = true;
    input.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "old-secret".into(),
    });
    let created = fixture.service.save_profile(input).unwrap();
    *fixture.store.fail_save.lock().unwrap() = true;
    let mut update = profile_input();
    update.id = Some(created.id.clone());
    update.authentication_enabled = true;
    update.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "new-secret".into(),
    });
    assert!(fixture.service.save_profile(update).is_err());
    assert_eq!(
        fixture
            .credentials
            .get(&created.id)
            .unwrap()
            .unwrap()
            .password,
        "old-secret"
    );
    assert_eq!(
        fixture.store.load().unwrap().profiles[0].host,
        "proxy.example.com"
    );
    assert_eq!(
        fixture.runtime.committed_active_ids.lock().unwrap().len(),
        3
    );
}

#[test]
fn rule_reorder_and_settings_reflect_applied_state() {
    let fixture = Fixture::new();
    fixture
        .service
        .replace_rules(vec![
            rule("first", "example.com"),
            rule("second", "other.net"),
        ])
        .unwrap();
    fixture
        .service
        .reorder_rules(&["second".into(), "first".into()])
        .unwrap();
    assert_eq!(fixture.service.list_rules().unwrap()[0].id, "second");
    let error = fixture
        .service
        .reorder_rules(&["second".into(), "second".into()])
        .unwrap_err();
    assert_eq!(error.fields[0].field, "rule_ids");
    let invalid = fixture
        .service
        .replace_rules(vec![rule("bad", "https://example.com")])
        .unwrap_err();
    assert_eq!(invalid.fields[0].field, "rules[0].target");
    let settings = fixture
        .service
        .update_settings(AppSettings {
            launch_at_login: true,
            diagnostic_retention: RetentionPolicy::Days7,
        })
        .unwrap();
    assert!(settings.launch_at_login);
    assert!(*fixture.startup.0.lock().unwrap());
    assert_eq!(
        fixture.service.settings().unwrap().diagnostic_retention,
        RetentionPolicy::Days7
    );
}

#[test]
fn import_requires_fresh_credentials_and_rolls_back_rejected_revision() {
    let fixture = Fixture::new();
    let mut input = profile_input();
    input.authentication_enabled = true;
    input.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "old-secret".into(),
    });
    let created = fixture.service.save_profile(input).unwrap();
    let exported = fixture.service.export().unwrap();
    let mut portable: serde_json::Value = serde_json::from_str(&exported).unwrap();
    portable["profiles"][0]["host"] = "changed.example.com".into();
    portable["settings"]["launch_at_login"] = true.into();
    let json = serde_json::to_string(&portable).unwrap();

    assert_eq!(
        fixture
            .service
            .import(&json, HashMap::new())
            .unwrap_err()
            .fields[0]
            .field,
        "profiles[0].credential"
    );
    assert_eq!(
        fixture.store.load().unwrap().profiles[0].host,
        "proxy.example.com"
    );
    assert_eq!(
        fixture
            .credentials
            .get(&created.id)
            .unwrap()
            .unwrap()
            .password,
        "old-secret"
    );

    *fixture.runtime.reject_next.lock().unwrap() = true;
    let updates = HashMap::from([(
        created.id.clone(),
        CredentialUpdate::Replace {
            username: "alice".into(),
            password: "new-secret".into(),
        },
    )]);
    assert!(fixture.service.import(&json, updates).is_err());
    assert_eq!(
        fixture.store.load().unwrap().profiles[0].host,
        "proxy.example.com"
    );
    assert_eq!(
        fixture
            .credentials
            .get(&created.id)
            .unwrap()
            .unwrap()
            .password,
        "old-secret"
    );
    assert!(!*fixture.startup.0.lock().unwrap());

    let updates = HashMap::from([(
        created.id.clone(),
        CredentialUpdate::Replace {
            username: "alice".into(),
            password: "new-secret".into(),
        },
    )]);
    fixture.service.import(&json, updates).unwrap();
    assert_eq!(
        fixture.store.load().unwrap().profiles[0].host,
        "changed.example.com"
    );
    assert_eq!(
        fixture
            .credentials
            .get(&created.id)
            .unwrap()
            .unwrap()
            .password,
        "new-secret"
    );
    assert!(*fixture.startup.0.lock().unwrap());
}
