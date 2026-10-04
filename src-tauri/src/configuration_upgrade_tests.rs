use super::*;
use crate::store::migrate_with_limit;
use rusqlite::Connection;

#[test]
fn previous_database_and_matching_credentials_are_required_for_safe_backout() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let database = root.join("config.sqlite3");
    let backup = root.join("schema-v3.sqlite3");
    let credentials = Credentials(root.join("keyring"));
    // The previous format stores a profile-ID reference in schema v3. Keep
    // synthetic secret backup separate, modelling an external protected backup.
    credentials
        .replace("profile", "old-user", "old-secret")
        .unwrap();
    let config = PersistedConfiguration {
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
    let previous = Connection::open(&database).unwrap();
    migrate_with_limit(&previous, 3, |_| Ok(())).unwrap();
    previous
        .execute(
            "INSERT INTO configuration VALUES (1, ?1)",
            [serde_json::to_string(&config).unwrap()],
        )
        .unwrap();
    drop(previous);
    fs::copy(&database, &backup).unwrap();
    let portable = crate::transfer::export_configuration_json(&config).unwrap();
    assert!(!portable.contains("old-secret"));
    assert!(!portable.contains("old-user"));

    let store = std::sync::Arc::new(SqliteConfigurationStore::open(&database).unwrap());
    assert_eq!(store.load().unwrap(), config);
    let lease = SessionLease::acquire(&uuid::Uuid::new_v4().to_string()).unwrap();
    let runtime = ManagedRuntime::from_lease(
        store.load().unwrap(),
        Box::new(Backend {
            root: root.to_owned(),
            credentials: Credentials(root.join("keyring")),
            #[cfg(windows)]
            cores: windows::Cores::default(),
        }),
        lease,
        RuntimeMode::Direct,
    )
    .unwrap()
    .with_configuration_revision(store.recovery_revision().unwrap());
    let service = ApplicationService::new(
        Box::new(store.clone()),
        Box::new(Credentials(root.join("keyring"))),
        Box::new(Startup(root.join("startup.json"))),
        Box::new(runtime),
    );
    service.upgrade_legacy_credentials().unwrap();
    let upgraded = store.load().unwrap();
    let reference = upgraded.profiles[0].credential_ref.clone().unwrap();
    assert_ne!(reference, "profile");
    assert!(credentials.get("profile").unwrap().is_none());
    assert_eq!(
        credentials.get(&reference).unwrap().unwrap().password,
        "old-secret"
    );
    assert_eq!(service.export().unwrap(), portable);
    drop(service);
    drop(store);

    // Replacing only the binary cannot bypass the old version gate, and must
    // not mutate the upgraded database.
    let before = fs::read(&database).unwrap();
    let previous_binary = Connection::open(&database).unwrap();
    assert!(migrate_with_limit(&previous_binary, 3, |_| Ok(())).is_err());
    drop(previous_binary);
    assert_eq!(fs::read(&database).unwrap(), before);

    // Restoring only the database points back to an entry the successful
    // upgrade retired. A password-free export cannot reconstruct that entry.
    fs::copy(&backup, &database).unwrap();
    let previous_binary = Connection::open(&database).unwrap();
    migrate_with_limit(&previous_binary, 3, |_| Ok(())).unwrap();
    let json: String = previous_binary
        .query_row(
            "SELECT document_json FROM configuration WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let restored: PersistedConfiguration = serde_json::from_str(&json).unwrap();
    assert!(
        crate::credentials::read_profile_credential(&credentials, &restored.profiles[0])
            .unwrap()
            .is_none()
    );
    // Model re-entry of original credentials or restoration of an external
    // protected credential backup; the application export is not that backup.
    credentials
        .replace("profile", "old-user", "old-secret")
        .unwrap();
    assert_eq!(
        crate::credentials::read_profile_credential(&credentials, &restored.profiles[0])
            .unwrap()
            .unwrap()
            .password,
        "old-secret"
    );
    assert_eq!(restored, config);
    assert_eq!(
        previous_binary
            .pragma_query_value::<i64, _>(None, "user_version", |row| row.get(0))
            .unwrap(),
        3
    );
}
