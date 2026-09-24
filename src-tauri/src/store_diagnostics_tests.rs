use super::*;

#[test]
fn filters_pages_and_clears_only_confirmed_diagnostics() {
    let store = SqliteConfigurationStore::open_in_memory().unwrap();
    let at = now_ms().unwrap();
    for (id, created_at_ms, severity) in [
        ("start", at - 3_000, "info"),
        ("switch", at - 2_000, "info"),
        ("rollback", at - 1_000, "error"),
    ] {
        store
            .record_diagnostic(&RuntimeDiagnostic {
                id: id.into(),
                created_at_ms,
                severity: severity.into(),
                summary: format!("runtime {id}"),
            })
            .unwrap();
    }
    let filter = DiagnosticFilter {
        from_ms: Some(at - 2_500),
        until_ms: Some(at),
        severity: None,
    };
    let first = store.list_diagnostics(&filter, 0, 1).unwrap();
    assert_eq!(first.total, 2);
    assert_eq!(first.items[0].id, "rollback");
    assert_eq!(first.next_offset, Some(1));
    let second = store.list_diagnostics(&filter, 1, 1).unwrap();
    assert_eq!(second.items[0].id, "switch");
    assert_eq!(second.next_offset, None);
    assert!(store.clear_diagnostics(&filter, false).is_err());
    assert_eq!(store.diagnostic_count().unwrap(), 3);
    assert_eq!(store.clear_diagnostics(&filter, true).unwrap(), 2);
    assert_eq!(store.diagnostic_count().unwrap(), 1);
    assert_eq!(
        store
            .list_diagnostics(&DiagnosticFilter::default(), 0, 10)
            .unwrap()
            .items[0]
            .id,
        "start"
    );
}

#[test]
fn rejects_invalid_filters_and_retains_permanent_cap() {
    let store = SqliteConfigurationStore::open_in_memory().unwrap();
    let invalid = DiagnosticFilter {
        from_ms: Some(5),
        until_ms: Some(5),
        severity: None,
    };
    assert!(store.list_diagnostics(&invalid, 0, 10).is_err());
    assert!(store.clear_diagnostics(&invalid, true).is_err());
    let invalid = DiagnosticFilter {
        severity: Some("success".into()),
        ..Default::default()
    };
    assert!(store.list_diagnostics(&invalid, 0, 10).is_err());
    assert!(store
        .list_diagnostics(&DiagnosticFilter::default(), 0, 101)
        .is_err());
}
