//! Explicit desktop acceptance: writes synthetic, uniquely named keyring entries
//! and briefly exercises the real current-user startup adapter.
use super::*;
use crate::{
    configuration_recovery::{RecoveryIntent, StartupChange, StartupEntry},
    credentials::OsCredentialStore,
    models::{PersistedConfiguration, ProxyProfile, ProxyProtocol},
    runtime_session::SessionLease,
    startup::SystemStartupAdapter,
    store::SqliteConfigurationStore,
};

struct Cleanup {
    references: Vec<String>,
    original: Option<StartupEntry>,
    owned: StartupEntry,
    external: StartupEntry,
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        for reference in &self.references {
            let _ = OsCredentialStore.delete(reference);
        }
        let startup = SystemStartupAdapter;
        if let Ok(current) = startup.read_entry() {
            if current.as_ref() == Some(&self.owned) || current.as_ref() == Some(&self.external) {
                let _ = startup.compare_exchange_entry(current.as_ref(), self.original.as_ref());
            }
        }
    }
}

#[test]
#[ignore = "explicit acceptance in an interactive Windows session; touches current-user startup"]
fn actual_keyring_and_startup_recover_both_commit_sides_and_preserve_external_value() {
    let startup = SystemStartupAdapter;
    let original = startup.read_entry().unwrap();
    assert!(
        original.is_none(),
        "acceptance requires an unused application startup entry"
    );
    let owned = startup.expected_entry().unwrap();
    let external = StartupEntry {
        value_type: 3, // Unique synthetic external REG_BINARY entry.
        bytes: uuid::Uuid::new_v4().as_bytes().to_vec(),
    };
    let mut cleanup = Cleanup {
        references: vec![],
        original,
        owned: owned.clone(),
        external: external.clone(),
    };
    for committed in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("configuration.sqlite3");
        let old_ref = format!("acceptance-old-{}", uuid::Uuid::new_v4());
        let new_ref = format!("acceptance-new-{}", uuid::Uuid::new_v4());
        cleanup
            .references
            .extend([old_ref.clone(), new_ref.clone()]);
        let credentials = OsCredentialStore;
        credentials
            .replace(&old_ref, "fixture-user", "fixture-old")
            .unwrap();
        credentials
            .replace(&new_ref, "fixture-user", "fixture-new")
            .unwrap();
        let previous = PersistedConfiguration {
            profiles: vec![ProxyProfile {
                id: uuid::Uuid::new_v4().to_string(),
                name: "OS acceptance".into(),
                protocol: ProxyProtocol::Http,
                host: "127.0.0.1".into(),
                port: 1080,
                authentication_enabled: true,
                credential_ref: Some(old_ref.clone()),
                enabled: true,
            }],
            ..Default::default()
        };
        let store = SqliteConfigurationStore::open(&path).unwrap();
        store.save(&previous).unwrap();
        let mut candidate = previous.clone();
        candidate.profiles[0].credential_ref = Some(new_ref.clone());
        candidate.settings.launch_at_login = true;
        let revision = store.recovery_revision().unwrap();
        let intent = RecoveryIntent {
            transaction_id: uuid::Uuid::new_v4().to_string(),
            previous_revision: revision,
            next_revision: revision + 1,
            previous: previous.clone(),
            candidate: candidate.clone(),
            staged_refs: vec![new_ref.clone()],
            retired_refs: vec![old_ref.clone()],
            startup: Some(StartupChange {
                original: None,
                expected: Some(owned.clone()),
            }),
        };
        store.begin_recovery(&intent).unwrap();
        startup.compare_exchange_entry(None, Some(&owned)).unwrap();
        if committed {
            store.commit_recovery(&intent.transaction_id).unwrap();
        }
        drop(store);
        let reopened = SqliteConfigurationStore::open(&path).unwrap();
        let lease = SessionLease::acquire(&uuid::Uuid::new_v4().to_string()).unwrap();
        startup
            .compare_exchange_entry(Some(&owned), Some(&external))
            .unwrap();
        assert!(
            recover_configuration_on_startup(&lease, &reopened, &credentials, &startup).is_err()
        );
        assert_eq!(startup.read_entry().unwrap(), Some(external.clone()));
        assert!(reopened.recovery_record().unwrap().is_some());
        assert!(credentials.get(&old_ref).unwrap().is_some());
        assert!(credentials.get(&new_ref).unwrap().is_some());
        startup
            .compare_exchange_entry(Some(&external), Some(&owned))
            .unwrap();
        recover_configuration_on_startup(&lease, &reopened, &credentials, &startup).unwrap();
        recover_configuration_on_startup(&lease, &reopened, &credentials, &startup).unwrap();
        assert_eq!(
            reopened.load().unwrap(),
            if committed { candidate } else { previous }
        );
        assert!(reopened.recovery_record().unwrap().is_none());
        let selected = if committed { &new_ref } else { &old_ref };
        let retired = if committed { &old_ref } else { &new_ref };
        let secret = credentials.get(selected).unwrap().unwrap();
        assert_eq!(
            secret.password,
            if committed {
                "fixture-new"
            } else {
                "fixture-old"
            }
        );
        assert!(credentials.get(retired).unwrap().is_none());
        if committed {
            startup.compare_exchange_entry(Some(&owned), None).unwrap();
        }
        assert!(startup.read_entry().unwrap().is_none());
    }
}
