use super::*;
use crate::store::ConfigurationStore;

fn event(id: &str, at: i64, operation: &str) -> RuntimeDiagnostic {
    RuntimeDiagnostic {
        id: id.into(),
        created_at_ms: at,
        severity: "error".into(),
        summary: "应用操作失败".into(),
        error_type: Some("runtime.start_failed".into()),
        operation: Some(operation.into()),
    }
}

#[test]
fn duplicate_identity_is_idempotent_but_distinct_failures_are_aggregated() -> Result<(), AppError> {
    let store = SqliteConfigurationStore::open_in_memory()?;
    let at = crate::store::diagnostic_now_ms()?;
    store.record_diagnostic(&event("a", at, "start"))?;
    store.record_diagnostic(&event("a", at + 9, "stop"))?;
    store.record_diagnostic(&event("b", at + 1, "start"))?;
    store.record_diagnostic(&event("c", at + 2, "stop"))?;
    let groups = store.diagnostic_groups(&DiagnosticFilter::default())?;
    assert_eq!(groups.len(), 2);
    let start = groups
        .iter()
        .find(|group| group.operation.as_deref() == Some("start"))
        .expect("start group");
    assert_eq!(start.occurrences, 2);
    assert_eq!((start.first_seen_ms, start.last_seen_ms), (at, at + 1));
    assert_eq!(store.diagnostic_count()?, 3);
    Ok(())
}

#[test]
fn export_contains_all_filtered_records_and_escapes_multiline_summaries() -> Result<(), AppError> {
    let store = std::sync::Arc::new(SqliteConfigurationStore::open_in_memory()?);
    let at = crate::store::diagnostic_now_ms()?;
    for index in 0..105 {
        let mut record = event(&format!("event-{index}"), at + index, "start");
        record.summary = "第一行\n第二行 \"quoted\"".into();
        store.record_diagnostic(&record)?;
    }
    let adapter: &dyn ConfigurationStore = &store;
    let exported = adapter.export_diagnostics(&DiagnosticFilter::default())?;
    assert_eq!(exported.lines().count(), 105);
    for line in exported.lines() {
        let json: serde_json::Value = serde_json::from_str(line).expect("valid JSON line");
        assert_eq!(json["error_type"], "runtime.start_failed");
        assert!(json["summary"].as_str().expect("summary").contains('\n'));
    }
    let filter = DiagnosticFilter {
        from_ms: Some(at + 100),
        until_ms: Some(at + 104),
        severity: Some("error".into()),
        search: None,
    };
    assert_eq!(adapter.export_diagnostics(&filter)?.lines().count(), 4);
    assert_eq!(adapter.diagnostic_groups(&filter)?[0].occurrences, 4);
    let empty = DiagnosticFilter {
        severity: Some("warning".into()),
        ..Default::default()
    };
    assert_eq!(adapter.export_diagnostics(&empty)?, "");
    assert!(adapter.diagnostic_groups(&empty)?.is_empty());
    assert!(adapter
        .export_diagnostics(&DiagnosticFilter {
            severity: Some("invalid".into()),
            ..Default::default()
        })
        .is_err());
    Ok(())
}

#[test]
fn migration_preserves_plain_records_and_extracts_legacy_error_types() -> Result<(), AppError> {
    let root = tempfile::tempdir().expect("temporary database");
    let path = root.path().join("legacy.sqlite3");
    let connection = rusqlite::Connection::open(&path).expect("legacy connection");
    crate::store::migrate_with_limit(&connection, 4, |_| Ok(()))?;
    connection
        .execute(
            "INSERT INTO runtime_diagnostics VALUES ('plain', 1, 'info', 'old event')",
            [],
        )
        .expect("plain event");
    connection
        .execute(
            "INSERT INTO runtime_diagnostics VALUES ('structured', 2, 'warning', ?1)",
            [r#"{"context":{"domain":"proxy","kind":"not_found","operation":"delete_profile"}}"#],
        )
        .expect("structured event");
    drop(connection);
    let store = SqliteConfigurationStore::open(&path)?;
    let page = store.list_diagnostics(&DiagnosticFilter::default(), 0, 10)?;
    assert_eq!(page.total, 2);
    assert_eq!(page.items[0].error_type.as_deref(), Some("proxy.not_found"));
    assert_eq!(page.items[0].operation.as_deref(), Some("delete_profile"));
    assert_eq!(page.items[1].error_type, None);
    assert_eq!(page.items[1].summary, "old event");
    assert_eq!(
        store.load()?,
        crate::models::PersistedConfiguration::default()
    );
    Ok(())
}

#[test]
fn literal_search_applies_consistently_to_pages_groups_export_and_clear() -> Result<(), AppError> {
    let store = SqliteConfigurationStore::open_in_memory()?;
    let at = crate::store::diagnostic_now_ms()?;
    let mut matching = event("Unique-42", at, "START");
    matching.summary = "输入 100%_ 是字面文本".into();
    store.record_diagnostic(&matching)?;
    let mut other = event("other", at, "stop");
    other.error_type = Some("runtime.stop_failed".into());
    store.record_diagnostic(&other)?;
    for query in ["unique-42", "100%_", "输入", "START"] {
        let filter = DiagnosticFilter {
            search: Some(query.into()),
            ..Default::default()
        };
        assert_eq!(store.list_diagnostics(&filter, 0, 10)?.total, 1);
        assert_eq!(store.diagnostic_groups(&filter)?[0].occurrences, 1);
        assert_eq!(store.export_diagnostics(&filter)?.lines().count(), 1);
    }
    let filter = DiagnosticFilter {
        search: Some("' OR 1=1 --".into()),
        ..Default::default()
    };
    assert_eq!(store.list_diagnostics(&filter, 0, 10)?.total, 0);
    let invalid = DiagnosticFilter {
        search: Some("x".repeat(129)),
        ..Default::default()
    };
    assert!(store.export_diagnostics(&invalid).is_err());
    let filter = DiagnosticFilter {
        search: Some("100%_".into()),
        ..Default::default()
    };
    assert_eq!(store.clear_diagnostics(&filter, true)?, 1);
    assert_eq!(store.diagnostic_count()?, 1);
    Ok(())
}
