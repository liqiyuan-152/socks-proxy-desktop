use super::*;
use crate::{
    configuration_recovery::{RecoveryIntent, StartupChange, StartupEntry},
    credentials::ProxyCredential,
    models::{PersistedConfiguration, ProxyProfile, ProxyProtocol},
    store::SqliteConfigurationStore,
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};

#[derive(Default)]
struct Credentials {
    entries: Mutex<HashMap<String, (String, String)>>,
    fail_delete: AtomicBool,
}
impl CredentialStore for Credentials {
    fn get(&self, reference: &str) -> Result<Option<ProxyCredential>, AppError> {
        Ok(self
            .entries
            .lock()
            .unwrap()
            .get(reference)
            .map(|(username, password)| ProxyCredential {
                username: username.clone(),
                password: password.clone(),
            }))
    }
    fn replace(&self, reference: &str, username: &str, password: &str) -> Result<(), AppError> {
        self.entries
            .lock()
            .unwrap()
            .insert(reference.into(), (username.into(), password.into()));
        Ok(())
    }
    fn delete(&self, reference: &str) -> Result<(), AppError> {
        if self.fail_delete.load(Ordering::SeqCst) {
            return Err(AppError::unavailable("cleanup"));
        }
        self.entries.lock().unwrap().remove(reference);
        Ok(())
    }
}

#[derive(Default)]
struct Startup(Mutex<Option<StartupEntry>>);
impl StartupAdapter for Startup {
    fn is_enabled(&self) -> Result<bool, AppError> {
        Ok(self.read_entry()?.as_ref() == Some(&self.expected_entry()?))
    }
    fn set_enabled(&self, _: bool) -> Result<(), AppError> {
        panic!("boolean recovery bypass");
    }
    fn read_entry(&self) -> Result<Option<StartupEntry>, AppError> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn expected_entry(&self) -> Result<StartupEntry, AppError> {
        Ok(StartupEntry {
            value_type: 1,
            bytes: vec![65, 0, 0, 0],
        })
    }
    fn write_entry(&self, entry: Option<&StartupEntry>) -> Result<(), AppError> {
        *self.0.lock().unwrap() = entry.cloned();
        Ok(())
    }
}

struct Fixture {
    directory: tempfile::TempDir,
    lease: SessionLease,
    credentials: Credentials,
    startup: Startup,
    intent: RecoveryIntent,
}
impl Fixture {
    fn new(committed: bool) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let store =
            SqliteConfigurationStore::open(directory.path().join("config.sqlite3")).unwrap();
        let previous = PersistedConfiguration {
            profiles: vec![ProxyProfile {
                id: "profile".into(),
                name: "Primary".into(),
                protocol: ProxyProtocol::Socks5,
                host: "proxy.example.com".into(),
                port: 1080,
                authentication_enabled: true,
                credential_ref: Some("profile".into()),
                enabled: true,
            }],
            ..Default::default()
        };
        store.save(&previous).unwrap();
        let mut candidate = previous.clone();
        candidate.profiles[0].credential_ref = Some("credential-v1-next".into());
        candidate.settings.launch_at_login = true;
        let startup = Startup::default();
        let intent = RecoveryIntent {
            transaction_id: uuid::Uuid::new_v4().to_string(),
            previous_revision: 1,
            next_revision: 2,
            previous,
            candidate,
            staged_refs: vec!["credential-v1-next".into()],
            retired_refs: vec!["profile".into()],
            startup: Some(StartupChange {
                original: None,
                expected: Some(startup.expected_entry().unwrap()),
            }),
        };
        store.begin_recovery(&intent).unwrap();
        let credentials = Credentials::default();
        credentials
            .replace("profile", "old-user", "old-secret")
            .unwrap();
        credentials
            .replace("credential-v1-next", "new-user", "new-secret")
            .unwrap();
        startup
            .write_entry(intent.startup.as_ref().unwrap().expected.as_ref())
            .unwrap();
        if committed {
            store.commit_recovery(&intent.transaction_id).unwrap();
        }
        drop(store);
        let lease = SessionLease::acquire(&uuid::Uuid::new_v4().to_string()).unwrap();
        Self {
            directory,
            lease,
            credentials,
            startup,
            intent,
        }
    }
    fn open(&self) -> SqliteConfigurationStore {
        SqliteConfigurationStore::open(self.directory.path().join("config.sqlite3")).unwrap()
    }
    fn recover(&self, store: &SqliteConfigurationStore) -> Result<(), AppError> {
        recover_configuration_on_startup(&self.lease, store, &self.credentials, &self.startup)
    }
}

#[test]
fn reopened_database_recovers_selected_commit_and_repeat_is_idempotent() {
    for committed in [false, true] {
        let fixture = Fixture::new(committed);
        let store = fixture.open();
        fixture.recover(&store).unwrap();
        fixture.recover(&store).unwrap();
        let expected = if committed {
            &fixture.intent.candidate
        } else {
            &fixture.intent.previous
        };
        assert_eq!(store.load().unwrap(), *expected);
        assert_eq!(
            store.recovery_revision().unwrap(),
            if committed { 2 } else { 1 }
        );
        assert_eq!(fixture.startup.is_enabled().unwrap(), committed);
        assert!(store.recovery_record().unwrap().is_none());
        assert_eq!(
            fixture.credentials.get("profile").unwrap().is_some(),
            !committed
        );
        assert_eq!(
            fixture
                .credentials
                .get("credential-v1-next")
                .unwrap()
                .is_some(),
            committed
        );
    }
}

#[test]
fn failed_cleanup_retains_journal_and_retries_same_commit_after_reopen() {
    for committed in [false, true] {
        let fixture = Fixture::new(committed);
        fixture
            .credentials
            .fail_delete
            .store(true, Ordering::SeqCst);
        let store = fixture.open();
        assert!(fixture.recover(&store).is_err());
        assert_eq!(
            store.recovery_record().unwrap().unwrap().committed,
            committed
        );
        assert_eq!(fixture.credentials.entries.lock().unwrap().len(), 2);
        drop(store);
        fixture
            .credentials
            .fail_delete
            .store(false, Ordering::SeqCst);
        fixture.recover(&fixture.open()).unwrap();
        assert_eq!(fixture.credentials.entries.lock().unwrap().len(), 1);
    }
}

#[test]
fn external_startup_value_is_preserved_until_ownership_is_resolved() {
    for committed in [false, true] {
        let fixture = Fixture::new(committed);
        let external = StartupEntry {
            value_type: 2,
            bytes: vec![66, 0],
        };
        fixture.startup.write_entry(Some(&external)).unwrap();
        let store = fixture.open();
        assert_eq!(
            fixture.recover(&store).unwrap_err().code,
            "startup_ownership"
        );
        assert_eq!(fixture.startup.read_entry().unwrap(), Some(external));
        assert!(store.recovery_record().unwrap().is_some());
        assert_eq!(fixture.credentials.entries.lock().unwrap().len(), 2);
        fixture
            .startup
            .write_entry(fixture.intent.startup.as_ref().unwrap().expected.as_ref())
            .unwrap();
        fixture.recover(&store).unwrap();
        assert!(store.recovery_record().unwrap().is_none());
    }
}

#[test]
fn missing_committed_replacement_does_not_destroy_old_secret_or_evidence() {
    let fixture = Fixture::new(true);
    fixture.credentials.delete("credential-v1-next").unwrap();
    let store = fixture.open();
    assert_eq!(
        fixture.recover(&store).unwrap_err().code,
        "configuration_recovery"
    );
    assert_eq!(store.load().unwrap(), fixture.intent.candidate);
    assert!(fixture.credentials.get("profile").unwrap().is_some());
    assert!(store.recovery_record().unwrap().unwrap().committed);
}
