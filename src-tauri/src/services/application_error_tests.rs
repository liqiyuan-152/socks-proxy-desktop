use super::*;
use crate::{
    services::application_service::tests::{
        FakeRuntime, Fixture, MemoryCredentials, MemoryStartup,
    },
    store::{DiagnosticFilter, SqliteConfigurationStore},
};
use std::sync::Arc;

#[test]
fn facade_records_error_identity_operation_and_sanitized_stack() -> Result<(), AppError> {
    let store = Arc::new(SqliteConfigurationStore::open_in_memory()?);
    let service = ApplicationService::new(
        Box::new(store.clone()),
        Box::new(Arc::new(MemoryCredentials::default())),
        Box::new(Arc::new(MemoryStartup::default())),
        Box::new(Arc::new(FakeRuntime::default())),
    );
    let error = service
        .delete_profile("missing")
        .expect_err("missing profile");
    let context = error.context.as_ref().expect("error context");
    assert_eq!(context.operation.as_deref(), Some("delete_profile"));
    assert_eq!(context.domain, ErrorDomain::Proxy);
    let page = store.list_diagnostics(&DiagnosticFilter::default(), 0, 10)?;
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].id, context.error_id);
    assert_eq!(page.items[0].severity, "warning");
    assert_eq!(page.items[0].error_type.as_deref(), Some("proxy.not_found"));
    assert_eq!(page.items[0].operation.as_deref(), Some("delete_profile"));
    service.record_error(&error, "应用操作失败");
    assert_eq!(store.diagnostic_count()?, 1);
    assert_eq!(
        service.diagnostic_groups(&DiagnosticFilter::default())?[0].occurrences,
        1
    );
    assert_eq!(
        service
            .export_runtime_diagnostics(&DiagnosticFilter::default())?
            .lines()
            .count(),
        1
    );
    let diagnostic: serde_json::Value =
        serde_json::from_str(&page.items[0].summary).expect("structured diagnostic");
    assert_eq!(diagnostic["context"]["error_id"], context.error_id);
    assert_eq!(diagnostic["context"]["kind"], "not_found");
    assert!(diagnostic["stack"].is_array());
    assert!(!page.items[0].summary.contains("/Users/"));
    Ok(())
}

#[test]
fn failed_diagnostic_persistence_never_replaces_business_error() {
    let fixture = Fixture::new();
    // The memory repository does not support diagnostics.
    let original = AppError::storage("原始存储失败");
    let result: Result<(), AppError> =
        fixture
            .service
            .finish(Err(original.clone()), ErrorDomain::Proxy, "save_profile");
    let error = result.expect_err("storage failure");
    assert_eq!(error.code, original.code);
    assert_eq!(error.message, original.message);
    assert_eq!(error.fields, original.fields);
    assert_eq!(error.context.expect("context").domain, ErrorDomain::Storage);
    let success =
        fixture
            .service
            .finish(Ok::<_, AppError>(42), ErrorDomain::Proxy, "list_profiles");
    assert_eq!(success.expect("successful operation"), 42);
    let copy = fixture
        .service
        .copy_active_connection_detail("missing")
        .expect_err("connection unavailable");
    assert_eq!(
        copy.context.expect("copy context").operation.as_deref(),
        Some("copy_active_connection_detail")
    );
}

#[test]
fn annotation_preserves_existing_identity_and_does_not_record_messages_or_fields() {
    let mut error = AppError::validation(vec![crate::error::FieldError {
        field: "private-field".into(),
        message: "password=secret-value".into(),
    }])
    .with_context(ErrorDomain::Credential, "invalid");
    error.message = "username=private-user".into();
    let id = error.context.as_ref().expect("context").error_id.clone();
    let error = annotate(error, ErrorDomain::Proxy, "save_profile");
    let context = error.context.as_ref().expect("context");
    assert_eq!(context.error_id, id);
    assert_eq!(context.domain, ErrorDomain::Credential);
    assert_eq!(context.kind, "invalid");
    let diagnostic = serde_json::to_string(&ErrorDiagnostic {
        summary: "应用操作失败",
        context,
        stack: &context.stack,
    })
    .expect("serialize safe diagnostic");
    for private in ["secret-value", "private-user", "private-field"] {
        assert!(!diagnostic.contains(private));
    }
}

#[test]
fn tracing_identity_is_persisted_and_preserved_across_error_boundaries(
) -> Result<(), Box<dyn std::error::Error>> {
    // 生产 subscriber 是进程级状态；独立子进程避免并行测试的 callsite 缓存干扰。
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "services::application_errors::tests::tracing_identity_child",
            "--ignored",
        ])
        .env("RUST_LOG", "info")
        .env_remove("SOCKS_PROXY_PROFILE")
        .output()?;
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
#[ignore = "isolated production tracing subscriber invoked by parent test"]
fn tracing_identity_child() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let guard = crate::tracing_setup::initialize(directory.path());
    let context = {
        let store = Arc::new(SqliteConfigurationStore::open_in_memory()?);
        let service = ApplicationService::new(
            Box::new(store.clone()),
            Box::new(Arc::new(MemoryCredentials::default())),
            Box::new(Arc::new(MemoryStartup::default())),
            Box::new(Arc::new(FakeRuntime::default())),
        );
        let error = service
            .delete_profile("missing")
            .expect_err("missing profile");
        let context = error.context.as_ref().expect("context").clone();
        assert!(uuid::Uuid::parse_str(context.trace_id.as_deref().expect("trace id")).is_ok());
        assert!(context.span_id.as_ref().is_some_and(|id| id.len() == 16));
        let next = annotate(error, ErrorDomain::Application, "retry");
        assert_eq!(
            next.context.as_ref().expect("context").trace_id,
            context.trace_id
        );
        assert_eq!(
            next.context.as_ref().expect("context").span_id,
            context.span_id
        );
        let page = store.list_diagnostics(&DiagnosticFilter::default(), 0, 10)?;
        let diagnostic: serde_json::Value =
            serde_json::from_str(&page.items[0].summary).expect("structured diagnostic");
        assert_eq!(
            diagnostic["context"]["trace_id"],
            context.trace_id.as_deref().expect("trace id")
        );
        assert_eq!(
            diagnostic["context"]["span_id"],
            context.span_id.as_deref().expect("span id")
        );
        context
    };
    drop(guard);
    let content = std::fs::read_to_string(directory.path().join("runtime.jsonl"))?;
    let events = content
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    let matching = events.iter().filter(|event| {
        event["fields"]["message"] == "application operation failed"
            && event["fields"]["error_id"] == context.error_id
    });
    let mut count = 0;
    for event in matching {
        assert_eq!(
            event["fields"]["trace_id"],
            context.trace_id.as_deref().unwrap()
        );
        assert_eq!(
            event["fields"]["span_id"],
            context.span_id.as_deref().unwrap()
        );
        count += 1;
    }
    assert_eq!(count, 2, "两个错误边界应记录相同标识");
    Ok(())
}
