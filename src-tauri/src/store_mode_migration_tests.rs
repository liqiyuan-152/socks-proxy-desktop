use super::*;

fn old_database(mode: &str, china: Value) -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    // Existing migration steps 1..6 create an authentic old schema.
    super::super::migrations::migrate_with_limit(&connection, 6, |_| Ok(())).unwrap();
    connection
        .execute("INSERT INTO selected_mode VALUES (1, ?1)", [mode])
        .unwrap();
    let mut document =
        serde_json::to_value(crate::models::PersistedConfiguration::default()).unwrap();
    document["schema_version"] = json!(2);
    document["china_direct_enabled"] = china;
    document.as_object_mut().unwrap().remove("runtime_mode");
    connection
        .execute(
            "INSERT INTO configuration VALUES (1, ?1)",
            [document.to_string()],
        )
        .unwrap();
    connection
}

#[test]
fn mode_migration_covers_all_legacy_combinations() {
    for mode in ["rules", "global", "direct"] {
        for enabled in [false, true] {
            let connection = old_database(mode, json!(enabled));
            let transaction = connection.unchecked_transaction().unwrap();
            upgrade(&transaction).unwrap();
            transaction.pragma_update(None, "user_version", 7).unwrap();
            transaction.commit().unwrap();
            let row: (String, Option<bool>, Option<String>) = connection
                .query_row(
                    "SELECT mode, rules_use_china_direct, rules_default_action FROM selected_mode",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .unwrap();
            assert_eq!(
                row,
                (
                    mode.into(),
                    (mode == "rules").then_some(enabled),
                    (mode == "rules").then(|| "proxy".into())
                )
            );
            let document: String = connection
                .query_row("SELECT document_json FROM configuration", [], |r| r.get(0))
                .unwrap();
            let document: Value = serde_json::from_str(&document).unwrap();
            assert!(document.get("china_direct_enabled").is_none());
            assert_eq!(document["schema_version"], 3);
            if mode == "rules" {
                assert_eq!(
                    document["runtime_mode"]["rules"]["use_china_direct"],
                    enabled
                );
                assert_eq!(document["runtime_mode"]["rules"]["default_action"], "proxy");
            } else {
                assert_eq!(document["runtime_mode"], mode);
            }
        }
    }
}

#[test]
fn mode_migration_failure_preserves_schema_document_and_version() {
    let connection = old_database("rules", json!("invalid"));
    let previous: String = connection
        .query_row("SELECT document_json FROM configuration", [], |r| r.get(0))
        .unwrap();
    {
        let transaction = connection.unchecked_transaction().unwrap();
        assert!(upgrade(&transaction).is_err());
    }
    assert_eq!(
        connection
            .pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        6
    );
    assert_eq!(
        connection
            .query_row::<String, _, _>("SELECT document_json FROM configuration", [], |r| r.get(0))
            .unwrap(),
        previous
    );
    assert!(connection
        .prepare("SELECT rules_default_action FROM selected_mode")
        .is_err());
}

#[test]
fn new_database_uses_v7_and_document_defaults_v3() {
    let connection = Connection::open_in_memory().unwrap();
    super::super::migrations::migrate(&connection).unwrap();
    assert_eq!(
        connection
            .pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        7
    );
    assert_eq!(
        crate::models::PersistedConfiguration::default().schema_version,
        3
    );
    assert!(connection
        .prepare("SELECT rules_use_china_direct, rules_default_action FROM selected_mode")
        .is_ok());
}

#[test]
fn previous_binary_gate_rejects_v7_without_changing_database() {
    let connection = Connection::open_in_memory().unwrap();
    super::super::migrations::migrate(&connection).unwrap();
    let before: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name = 'selected_mode'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(super::super::migrations::migrate_with_limit(&connection, 6, |_| Ok(())).is_err());
    assert_eq!(
        connection
            .pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        7
    );
    assert_eq!(
        connection
            .query_row::<String, _, _>(
                "SELECT sql FROM sqlite_master WHERE name = 'selected_mode'",
                [],
                |r| r.get(0)
            )
            .unwrap(),
        before
    );
}

#[test]
fn real_v6_files_reopen_with_v3_configuration_and_preserved_mode_parameters() {
    use crate::store::{ConfigurationStore, SqliteConfigurationStore};
    for mode in ["rules", "global", "direct"] {
        for enabled in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("upgrade.sqlite3");
            let connection = Connection::open(&path).unwrap();
            super::super::migrations::migrate_with_limit(&connection, 6, |_| Ok(())).unwrap();
            connection
                .execute("INSERT INTO selected_mode VALUES (1, ?1)", [mode])
                .unwrap();
            let mut document =
                serde_json::to_value(crate::models::PersistedConfiguration::default()).unwrap();
            document["schema_version"] = json!(2);
            document["china_direct_enabled"] = json!(enabled);
            document.as_object_mut().unwrap().remove("runtime_mode");
            connection
                .execute(
                    "INSERT INTO configuration VALUES (1, ?1)",
                    [document.to_string()],
                )
                .unwrap();
            drop(connection);
            let store = SqliteConfigurationStore::open(&path).unwrap();
            let config = store.load().unwrap();
            let expected = match mode {
                "rules" => RuntimeMode::Rules {
                    use_china_direct: enabled,
                    default_action: crate::models::RuleAction::Proxy,
                },
                "global" => RuntimeMode::Global,
                _ => RuntimeMode::Direct,
            };
            assert_eq!(config.schema_version, 3);
            assert_eq!(config.runtime_mode, expected);
            assert_eq!(store.load_mode().unwrap(), expected);
            drop(store);
            assert_eq!(
                SqliteConfigurationStore::open(&path)
                    .unwrap()
                    .load()
                    .unwrap(),
                config
            );
        }
    }
}

#[test]
fn failure_after_document_conversion_rolls_back_all_migration_changes() {
    let connection = old_database("rules", json!(true));
    connection
        .execute(
            "INSERT INTO configuration_recovery VALUES (1, ?1, 0)",
            ["{}"],
        )
        .unwrap();
    let previous: String = connection
        .query_row("SELECT document_json FROM configuration", [], |r| r.get(0))
        .unwrap();
    assert!(super::super::migrations::migrate(&connection).is_err());
    assert_eq!(
        connection
            .pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        6
    );
    assert_eq!(
        connection
            .query_row::<String, _, _>("SELECT document_json FROM configuration", [], |r| r.get(0))
            .unwrap(),
        previous
    );
    assert!(connection
        .prepare("SELECT rules_use_china_direct FROM selected_mode")
        .is_err());
    assert_eq!(
        connection
            .query_row::<String, _, _>("SELECT intent_json FROM configuration_recovery", [], |r| r
                .get(0))
            .unwrap(),
        "{}"
    );
}
