use super::*;
use crate::models::{ProxyProfile, ProxyProtocol};

fn intent(store: &SqliteConfigurationStore) -> RecoveryIntent {
    let previous = store.load().unwrap();
    let mut candidate = previous.clone();
    candidate.profiles[0].credential_ref = Some(format!("credential-v1-{}", uuid::Uuid::new_v4()));
    let previous_revision = store.recovery_revision().unwrap();
    RecoveryIntent {
        transaction_id: uuid::Uuid::new_v4().to_string(),
        previous_revision,
        next_revision: previous_revision + 1,
        staged_refs: vec![candidate.profiles[0].credential_ref.clone().unwrap()],
        retired_refs: vec![previous.profiles[0].credential_ref.clone().unwrap()],
        previous,
        candidate,
        startup: None,
    }
}

fn configure(store: &SqliteConfigurationStore) {
    let configuration = PersistedConfiguration {
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
        active_profile_id: Some("profile".into()),
        ..Default::default()
    };
    store.save(&configuration).unwrap();
}

#[test]
fn durable_intent_commit_and_cleanup_survive_reopen_and_are_idempotent() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.sqlite3");
    let store = SqliteConfigurationStore::open(&path).unwrap();
    configure(&store);
    let intent = intent(&store);
    store.begin_recovery(&intent).unwrap();
    assert!(store.save(&intent.candidate).is_err());
    assert!(store.save_mode(RuntimeMode::Global).is_err());
    assert_eq!(store.load_mode().unwrap(), RuntimeMode::Direct);
    assert!(store.begin_recovery(&intent).is_err());
    drop(store);
    let store = SqliteConfigurationStore::open(&path).unwrap();
    assert_eq!(store.load().unwrap(), intent.previous);
    assert_eq!(
        store.recovery_record().unwrap().unwrap(),
        RecoveryRecord {
            intent: intent.clone(),
            committed: false
        }
    );
    store.commit_recovery(&intent.transaction_id).unwrap();
    store.commit_recovery(&intent.transaction_id).unwrap();
    drop(store);
    let store = SqliteConfigurationStore::open(&path).unwrap();
    assert_eq!(store.load().unwrap(), intent.candidate);
    assert_eq!(store.recovery_revision().unwrap(), intent.next_revision);
    assert!(store.recovery_record().unwrap().unwrap().committed);
    assert!(store.save_mode(RuntimeMode::Global).is_err());
    assert!(store.clear_recovery("different-transaction").is_err());
    store.clear_recovery(&intent.transaction_id).unwrap();
    store.clear_recovery(&intent.transaction_id).unwrap();
    assert!(store.recovery_record().unwrap().is_none());
    store.save_mode(RuntimeMode::Global).unwrap();
    assert_eq!(store.load_mode().unwrap(), RuntimeMode::Global);
}

#[test]
#[ignore = "isolated process child, invoked by commit interruption test"]
fn interrupted_commit_child() {
    let path = std::env::var("RECOVERY_TEST_DATABASE").unwrap();
    let store = SqliteConfigurationStore::open(path).unwrap();
    let id = store
        .recovery_record()
        .unwrap()
        .unwrap()
        .intent
        .transaction_id;
    let _ = store.commit_recovery_with_hook(&id, || std::process::exit(88));
    panic!("interruption point not reached");
}

#[test]
fn interrupted_commit_does_not_split_configuration_references_revision_and_marker() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.sqlite3");
    let store = SqliteConfigurationStore::open(&path).unwrap();
    configure(&store);
    let intent = intent(&store);
    store.begin_recovery(&intent).unwrap();
    drop(store);
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "store::recovery::tests::interrupted_commit_child",
            "--ignored",
        ])
        .env("RECOVERY_TEST_DATABASE", &path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(88), "{output:?}");
    let store = SqliteConfigurationStore::open(&path).unwrap();
    assert_eq!(store.load().unwrap(), intent.previous);
    assert_eq!(store.recovery_revision().unwrap(), intent.previous_revision);
    assert!(!store.recovery_record().unwrap().unwrap().committed);
    store.commit_recovery(&intent.transaction_id).unwrap();
    assert_eq!(store.load().unwrap(), intent.candidate);
    assert_eq!(store.recovery_revision().unwrap(), intent.next_revision);
    assert!(store.recovery_record().unwrap().unwrap().committed);
}

#[test]
fn returned_commit_failure_keeps_intent_and_old_configuration() {
    let store = SqliteConfigurationStore::open_in_memory().unwrap();
    configure(&store);
    let intent = intent(&store);
    store.begin_recovery(&intent).unwrap();
    assert!(store
        .commit_recovery_with_hook(&intent.transaction_id, || Err(AppError::storage(
            "injected"
        )))
        .is_err());
    assert_eq!(store.load().unwrap(), intent.previous);
    assert_eq!(store.recovery_revision().unwrap(), intent.previous_revision);
    assert!(!store.recovery_record().unwrap().unwrap().committed);
    store.clear_recovery(&intent.transaction_id).unwrap();
    assert_eq!(store.load().unwrap(), intent.previous);
}

#[test]
fn rejects_stale_and_unsafe_cleanup_records_and_credential_fields() {
    let store = SqliteConfigurationStore::open_in_memory().unwrap();
    configure(&store);
    let intent = intent(&store);
    let mut stale = intent.clone();
    stale.previous_revision -= 1;
    stale.next_revision -= 1;
    assert!(store.begin_recovery(&stale).is_err());
    let mut unsafe_record = intent.clone();
    unsafe_record.retired_refs = unsafe_record.staged_refs.clone();
    assert!(store.begin_recovery(&unsafe_record).is_err());
    let mut unsafe_record = intent.clone();
    unsafe_record.staged_refs.clear();
    assert!(store.begin_recovery(&unsafe_record).is_err());
    let mut json = serde_json::to_value(&intent).unwrap();
    json["password"] = "must-not-enter-journal".into();
    assert!(serde_json::from_value::<RecoveryIntent>(json).is_err());
    let serialized = serde_json::to_string(&intent).unwrap();
    for forbidden in [
        "password",
        "username",
        "control_secret",
        "must-not-enter-journal",
    ] {
        assert!(!serialized.contains(forbidden));
    }
}
