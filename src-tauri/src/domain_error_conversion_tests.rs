use super::*;

fn assert_base_eq(actual: AppError, expected: AppError) {
    assert_eq!(actual.code, expected.code);
    assert_eq!(actual.message, expected.message);
    assert_eq!(actual.fields, expected.fields);
    assert!(actual.context.is_some());
}

fn validation() -> ValidationError {
    ValidationError {
        fields: vec![FieldError {
            field: "port".into(),
            message: "端口无效".into(),
        }],
    }
}

#[test]
fn proxy_conversion_preserves_identity_and_all_references() {
    let missing = AppError::from(ProxyError::NotFound {
        id: "profile-1".into(),
    });
    assert_eq!(missing.code, "proxy_not_found");
    assert_eq!(missing.message, "代理档案不存在: profile-1");
    let references = vec![
        ReferenceLocation {
            field: "default_profile_id".into(),
            description: "默认代理".into(),
        },
        ReferenceLocation {
            field: "rules[0].proxy_profile_id".into(),
            description: "规则一".into(),
        },
    ];
    let in_use = AppError::from(ProxyError::InUse { references });
    assert_eq!(in_use.code, "proxy_in_use");
    assert_eq!(in_use.fields.len(), 2);
    assert_eq!(in_use.fields[1].field, "rules[0].proxy_profile_id");
    assert_eq!(in_use.fields[1].message, "规则一");
}

#[test]
fn nested_validation_conversion_keeps_fields() {
    let expected = AppError::validation(validation().fields);
    for actual in [
        AppError::from(validation()),
        AppError::from(CredentialError::Invalid(validation())),
        AppError::from(ProxyError::CredentialInvalid(CredentialError::Invalid(
            validation(),
        ))),
        AppError::from(ProxyError::ValidationFailed(validation())),
        AppError::from(RoutingError::RuleValidationFailed(validation())),
        AppError::from(RoutingError::InvalidTarget(validation())),
    ] {
        assert_base_eq(actual, expected.clone());
    }
}

#[test]
fn credential_status_has_no_credential_contents() {
    for issue in [CredentialError::Missing, CredentialError::Disabled] {
        let expected = issue.to_string();
        let actual = AppError::from(issue);
        assert_eq!(actual.code, "credential_error");
        assert_eq!(actual.message, expected);
        assert!(actual.fields.is_empty());
    }
}

#[test]
fn storage_conversion_keeps_operation_message_and_stable_code() {
    for issue in [
        StorageError::ReadFailed,
        StorageError::WriteFailed,
        StorageError::TransactionFailed,
        StorageError::CorruptedData,
        StorageError::UnsupportedVersion,
        StorageError::LockUnavailable,
    ] {
        let expected = AppError::storage(issue.to_string());
        assert_base_eq(AppError::from(issue.clone()), expected.clone());
        assert_base_eq(
            AppError::from(ProxyError::StorageFailed(issue.clone())),
            expected.clone(),
        );
        assert_base_eq(
            AppError::from(RoutingError::StorageFailed(issue.clone())),
            expected.clone(),
        );
        assert_base_eq(
            AppError::from(RuntimeError::StorageFailed(issue)),
            expected.clone(),
        );
    }
}

#[test]
fn adapter_conversion_is_lossless_in_every_domain() {
    let original = AppError::validation(validation().fields);
    for actual in [
        AppError::from(CredentialError::Unavailable(original.clone())),
        AppError::from(StorageError::OperationFailed(original.clone())),
        AppError::from(ProxyError::OperationFailed(original.clone())),
        AppError::from(RoutingError::OperationFailed(original.clone())),
        AppError::from(RuntimeError::OperationFailed(original.clone())),
    ] {
        assert_base_eq(actual.clone(), original.clone());
        let mut actual = actual;
        actual.context = None;
        assert_eq!(
            serde_json::to_value(actual).expect("serialize"),
            serde_json::json!({
                "code": "validation_error", "message": "配置包含无效字段",
                "fields": [{ "field": "port", "message": "端口无效" }],
            })
        );
    }
}

#[test]
fn recovery_and_china_rules_conversion_are_explicit() {
    let recovery = crate::configuration_recovery::recovery_error();
    assert_base_eq(
        AppError::from(ProxyError::RecoveryInProgress),
        recovery.clone(),
    );
    assert_base_eq(
        AppError::from(RoutingError::RecoveryInProgress),
        recovery.clone(),
    );
    assert_base_eq(
        AppError::from(RuntimeError::RecoveryInProgress),
        recovery.clone(),
    );
    let china = AppError::from(RoutingError::ChinaRulesUnavailable);
    assert_eq!(china.code, "china_rules_unavailable");
    assert!(china.message.contains("规则集"));
}

#[test]
fn runtime_conversion_identifies_failure_without_leaking_internal_details() {
    let cases = [
        (
            RuntimeError::ProcessStartFailed {
                reason: "启动超时".into(),
            },
            "process_start_failed",
            "进程启动失败: 启动超时",
        ),
        (
            RuntimeError::InvalidTransition {
                from: RuntimePhase::Stopped,
                to: RuntimePhase::Running,
            },
            "invalid_transition",
            "无效的状态转换: Stopped -> Running",
        ),
        (
            RuntimeError::SystemProxyFailed(AppError::storage("private database path")),
            "system_proxy_failed",
            "系统代理设置失败",
        ),
        (
            RuntimeError::InvariantViolated("private internal state".into()),
            "runtime_invariant_violated",
            "运行时状态异常，请停止代理并执行网络恢复",
        ),
        (
            RuntimeError::Unavailable,
            "unavailable",
            "当前平台不支持此运行时操作",
        ),
    ];
    for (issue, code, message) in cases {
        let actual = AppError::from(issue);
        assert_eq!(actual.code, code);
        assert_eq!(actual.message, message);
        assert!(actual.fields.is_empty());
    }
}
