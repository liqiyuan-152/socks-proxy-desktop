use super::*;

#[test]
fn enrichment_keeps_wire_fields_and_assigns_unique_identity() {
    let original = AppError::validation(vec![crate::error::FieldError {
        field: "port".into(),
        message: "端口无效".into(),
    }]);
    let first = original
        .clone()
        .with_context(ErrorDomain::Proxy, "validation_failed");
    let second = original
        .clone()
        .with_context(ErrorDomain::Proxy, "validation_failed");
    assert_eq!(first.code, original.code);
    assert_eq!(first.message, original.message);
    assert_eq!(first.fields, original.fields);
    let context = first.context.as_ref().expect("context");
    assert!(uuid::Uuid::parse_str(&context.error_id).is_ok());
    assert!(context.timestamp_ms > 0);
    assert_ne!(
        context.error_id,
        second.context.expect("second context").error_id
    );
    assert_eq!(context.domain, ErrorDomain::Proxy);
    assert_eq!(context.kind, "validation_failed");
    assert!(context.recovery_suggestion.contains("字段"));
    assert_eq!(
        first
            .clone()
            .with_context(ErrorDomain::Runtime, "operation_failed"),
        first
    );
}

#[test]
fn wire_context_excludes_stack_and_preserves_legacy_shape_without_context() {
    let legacy = AppError::storage("本地存储不可用");
    assert_eq!(
        serde_json::to_value(&legacy).expect("serialize"),
        serde_json::json!({
            "code": "storage_error", "message": "本地存储不可用", "fields": [],
        })
    );
    let mut error = legacy.with_context(ErrorDomain::Storage, "write_failed");
    error.context.as_mut().expect("context").stack = vec!["private-source-location".into()];
    let value = serde_json::to_value(&error).expect("serialize enriched");
    assert_eq!(value["context"]["domain"], "storage");
    assert_eq!(value["context"]["kind"], "write_failed");
    assert!(value["context"].get("operation").is_none());
    assert!(value["context"].get("stack").is_none());
    assert!(!value.to_string().contains("private-source-location"));
}

#[test]
fn stack_collection_excludes_addresses_paths_and_limits_frame_count() {
    let trace = "0: socks_proxy_lib::services::save\n   at /Users/private/source.rs:42\n1: /Users/private/plugin\n2: 0x123@module\n3: socks_proxy_lib::error::finish";
    assert_eq!(
        stack_symbols(trace),
        vec![
            "socks_proxy_lib::services::save",
            "socks_proxy_lib::error::finish"
        ]
    );
    let trace = (0..30)
        .map(|n| format!("{n}: module::frame{n}\n"))
        .collect::<String>();
    assert_eq!(stack_symbols(&trace).len(), 16);
    assert!(stack_symbols(&format!("0: {}", "x".repeat(257))).is_empty());
}

#[test]
fn suggestions_cover_known_recovery_paths_and_unknown_errors() {
    for (code, word) in [
        ("proxy_not_found", "刷新"),
        ("not_found", "刷新"),
        ("proxy_in_use", "引用"),
        ("credential_error", "凭据"),
        ("validation_error", "字段"),
        ("china_rules_unavailable", "规则集"),
        ("configuration_recovery", "恢复"),
        ("rollback_failed", "恢复"),
        ("system_proxy_failed", "恢复"),
        ("runtime_invariant_violated", "恢复"),
        ("process_start_failed", "端口"),
        ("core_process_error", "端口"),
        ("runtime_error", "端口"),
        ("storage_error", "存储"),
        ("runtime_already_owned", "另一"),
        ("invalid_transition", "等待"),
        ("unavailable", "平台"),
        ("unknown", "诊断"),
    ] {
        assert!(recovery_suggestion(code).contains(word), "{code}");
    }
}
