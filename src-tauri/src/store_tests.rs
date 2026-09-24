use super::*;
use crate::models::{AppSettings, ProxyProfile, ProxyProtocol, RoutingRule};
use rusqlite::params;

fn configuration() -> PersistedConfiguration {
    PersistedConfiguration {
        schema_version: crate::models::CONFIG_SCHEMA_VERSION,
        profiles: vec![ProxyProfile {
            id: "profile-stable-id".into(),
            name: "Primary".into(),
            protocol: ProxyProtocol::Socks5,
            host: "127.0.0.1".into(),
            port: 1080,
            authentication_enabled: false,
            credential_ref: None,
            enabled: true,
        }],
        rules: vec![RoutingRule {
            id: "rule-stable-id".into(),
            name: "Internal".into(),
            matcher: crate::models::RuleMatcher::DomainSuffix,
            target: "example.com".into(),
            port_start: None,
            port_end: None,
            action: crate::models::RuleAction::Direct,
            proxy_profile_id: None,
            enabled: true,
        }],
        active_profile_id: Some("profile-stable-id".into()),
        china_direct_enabled: false,
        legacy_unresolved_rule_ids: Vec::new(),
        settings: AppSettings {
            launch_at_login: true,
            diagnostic_retention: RetentionPolicy::Days90,
            latency_test_url: crate::models::default_latency_test_url(),
        },
    }
}

#[test]
fn migrates_empty_database_and_persists_configuration_across_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.sqlite3");
    let expected = configuration();
    {
        let store = SqliteConfigurationStore::open(&path).unwrap();
        store.save(&expected).unwrap();
        assert_eq!(store.load().unwrap(), expected);
    }
    let reopened = SqliteConfigurationStore::open(&path).unwrap();
    assert_eq!(reopened.load().unwrap(), expected);
}

#[test]
fn selected_mode_defaults_to_direct_and_survives_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.sqlite3");
    let store = SqliteConfigurationStore::open(&path).unwrap();
    assert_eq!(store.load_mode().unwrap(), RuntimeMode::Direct);
    store.save_mode(RuntimeMode::Global).unwrap();
    store.save(&configuration()).unwrap();
    drop(store);

    let reopened = SqliteConfigurationStore::open(&path).unwrap();
    assert_eq!(reopened.load_mode().unwrap(), RuntimeMode::Global);
    reopened.save_mode(RuntimeMode::Rules).unwrap();
    assert_eq!(reopened.load_mode().unwrap(), RuntimeMode::Rules);
}

#[test]
fn upgrades_existing_v2_database_without_changing_configuration() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.sqlite3");
    let existing = configuration();
    let connection = Connection::open(&path).unwrap();
    connection
            .execute_batch(
                "CREATE TABLE configuration (id INTEGER PRIMARY KEY, document_json TEXT NOT NULL);
                 CREATE TABLE runtime_diagnostics (id TEXT PRIMARY KEY, created_at_ms INTEGER NOT NULL,
                     severity TEXT NOT NULL, summary TEXT NOT NULL);
                 CREATE TABLE proxy_ownership (id INTEGER PRIMARY KEY, record_json TEXT NOT NULL);
                 PRAGMA user_version = 2;",
            )
            .unwrap();
    connection
        .execute(
            "INSERT INTO configuration VALUES (1, ?1)",
            [serde_json::to_string(&existing).unwrap()],
        )
        .unwrap();
    drop(connection);

    let upgraded = SqliteConfigurationStore::open(&path).unwrap();
    assert_eq!(upgraded.load().unwrap(), existing);
    assert_eq!(upgraded.load_mode().unwrap(), RuntimeMode::Direct);
}

#[test]
fn reads_v1_document_and_binds_ordered_proxy_rules_to_old_active_profile() {
    let store = SqliteConfigurationStore::open_in_memory().unwrap();
    let mut old = serde_json::to_value(configuration()).unwrap();
    old["schema_version"] = 1.into();
    old["active_profile_id"] = old["default_profile_id"].take();
    old.as_object_mut().unwrap().remove("default_profile_id");
    old.as_object_mut().unwrap().remove("china_direct_enabled");
    let mut second = old["rules"][0].clone();
    second["id"] = "proxy-rule".into();
    second["name"] = "Proxy".into();
    second["action"] = "proxy".into();
    old["rules"].as_array_mut().unwrap().push(second);
    for rule in old["rules"].as_array_mut().unwrap() {
        rule.as_object_mut().unwrap().remove("proxy_profile_id");
    }
    store
        .connection
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO configuration VALUES (1, ?1)",
            [old.to_string()],
        )
        .unwrap();

    let migrated = store.load().unwrap();
    assert_eq!(migrated.schema_version, 2);
    assert_eq!(
        migrated.active_profile_id.as_deref(),
        Some("profile-stable-id")
    );
    assert_eq!(
        migrated
            .rules
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        ["rule-stable-id", "proxy-rule"]
    );
    assert_eq!(
        migrated.rules[1].proxy_profile_id.as_deref(),
        Some("profile-stable-id")
    );
    assert!(!migrated.china_direct_enabled);
    store.save(&migrated).unwrap();
    assert_eq!(store.load().unwrap(), migrated);
}

#[test]
fn v1_without_active_profile_keeps_proxy_rule_pending_until_repaired() {
    let store = SqliteConfigurationStore::open_in_memory().unwrap();
    let mut old = serde_json::to_value(configuration()).unwrap();
    old["schema_version"] = 1.into();
    old["active_profile_id"] = serde_json::Value::Null;
    old.as_object_mut().unwrap().remove("default_profile_id");
    old["rules"][0]["action"] = "proxy".into();
    old["rules"][0]
        .as_object_mut()
        .unwrap()
        .remove("proxy_profile_id");
    store
        .connection
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO configuration VALUES (1, ?1)",
            [old.to_string()],
        )
        .unwrap();

    let migrated = store.load().unwrap();
    assert_eq!(migrated.legacy_unresolved_rule_ids, ["rule-stable-id"]);
    assert_eq!(migrated.rules[0].action, crate::models::RuleAction::Proxy);
    assert_eq!(migrated.rules[0].proxy_profile_id, None);
    assert_eq!(
        migrated.validate_runtime_rules().unwrap_err().fields[0].field,
        "rules[0].proxy_profile_id"
    );
    store.save(&migrated).unwrap();
    assert_eq!(store.load().unwrap(), migrated);
    let mut repaired = migrated;
    repaired.rules[0].proxy_profile_id = Some("profile-stable-id".into());
    repaired.legacy_unresolved_rule_ids.clear();
    store.save(&repaired).unwrap();
    assert!(store.load().unwrap().validate_runtime_rules().is_ok());
}

#[test]
fn migration_sets_supported_schema_version_and_defaults_settings() {
    let store = SqliteConfigurationStore::open_in_memory().unwrap();
    let connection = store.connection.lock().unwrap();
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, DATABASE_SCHEMA_VERSION);
    drop(connection);
    assert_eq!(
        store.load().unwrap().settings.diagnostic_retention,
        RetentionPolicy::Days30
    );
}

#[test]
fn retention_applies_time_windows_and_permanent_capacity() {
    let store = SqliteConfigurationStore::open_in_memory().unwrap();
    let now = now_ms().unwrap();
    {
        let connection = store.connection.lock().unwrap();
        for (index, days_ago) in [0, 20, 60, 120].iter().enumerate() {
            connection
                .execute(
                    "INSERT INTO runtime_diagnostics VALUES (?1, ?2, 'info', 'runtime event')",
                    params![
                        format!("diag-{index}"),
                        now - days_ago * 24 * 60 * 60 * 1000
                    ],
                )
                .unwrap();
        }
    }
    assert_eq!(
        store
            .apply_diagnostic_retention(RetentionPolicy::Days90, now)
            .unwrap(),
        1
    );
    assert_eq!(
        store
            .apply_diagnostic_retention(RetentionPolicy::Days30, now)
            .unwrap(),
        1
    );
    assert_eq!(
        store
            .apply_diagnostic_retention(RetentionPolicy::Days7, now)
            .unwrap(),
        1
    );
    assert_eq!(store.diagnostic_count().unwrap(), 1);
}

#[test]
fn permanent_retention_caps_records_during_insert() {
    let store = SqliteConfigurationStore::open_in_memory().unwrap();
    let mut config = configuration();
    config.settings.diagnostic_retention = RetentionPolicy::Permanent;
    store.save(&config).unwrap();
    let now = now_ms().unwrap();
    {
        let connection = store.connection.lock().unwrap();
        connection
            .execute(
                "WITH RECURSIVE seq(n) AS (
                    SELECT 0 UNION ALL SELECT n + 1 FROM seq WHERE n < 99999
                ) INSERT INTO runtime_diagnostics (id, created_at_ms, severity, summary)
                  SELECT 'diag-' || n, ?1, 'info', 'runtime event' FROM seq",
                [now - 1],
            )
            .unwrap();
    }
    store
        .record_diagnostic(&RuntimeDiagnostic {
            id: "newest".into(),
            created_at_ms: now,
            severity: "info".into(),
            summary: "runtime event".into(),
        })
        .unwrap();
    assert_eq!(store.diagnostic_count().unwrap(), 100_000);
    let connection = store.connection.lock().unwrap();
    let newest: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM runtime_diagnostics WHERE id = 'newest'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(newest, 1);
}

#[test]
fn invalid_save_preserves_existing_configuration() {
    let store = SqliteConfigurationStore::open_in_memory().unwrap();
    let expected = configuration();
    store.save(&expected).unwrap();
    let mut invalid = expected.clone();
    invalid.profiles[0].port = 0;
    assert_eq!(
        store.save(&invalid).unwrap_err().fields[0].field,
        "profiles[0].port"
    );
    assert_eq!(store.load().unwrap(), expected);
}
