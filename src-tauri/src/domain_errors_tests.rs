use super::*;
use std::error::Error;

fn validation() -> ValidationError {
    ValidationError {
        fields: vec![FieldError {
            field: "port".into(),
            message: "端口必须大于零".into(),
        }],
    }
}

#[test]
fn nested_validation_preserves_field_details_and_source() {
    let issue = validation();
    let proxy = ProxyError::from(issue.clone());
    assert!(matches!(&proxy, ProxyError::ValidationFailed(value) if value == &issue));
    assert_eq!(
        proxy.source().expect("validation source").to_string(),
        issue.to_string()
    );
    let routing = RoutingError::from(issue.clone());
    assert!(matches!(routing, RoutingError::RuleValidationFailed(value) if value == issue));
    let credential = ProxyError::from(CredentialError::from(validation()));
    assert!(matches!(
        credential,
        ProxyError::CredentialInvalid(CredentialError::Invalid(_))
    ));
}

#[test]
fn deletion_error_retains_every_reference() {
    let references = vec![
        ReferenceLocation {
            field: "default_profile_id".into(),
            description: "默认代理".into(),
        },
        ReferenceLocation {
            field: "rules[0].proxy_profile_id".into(),
            description: "用户规则".into(),
        },
    ];
    let error = ProxyError::InUse {
        references: references.clone(),
    };
    assert_eq!(error.to_string(), "代理正在被使用，无法删除");
    assert!(matches!(error, ProxyError::InUse { references: actual } if actual == references));
}

#[test]
fn storage_failures_remain_distinct_in_all_domains() {
    for error in [
        StorageError::ReadFailed,
        StorageError::WriteFailed,
        StorageError::TransactionFailed,
        StorageError::CorruptedData,
        StorageError::UnsupportedVersion,
        StorageError::LockUnavailable,
    ] {
        assert!(
            matches!(ProxyError::from(error.clone()), ProxyError::StorageFailed(value) if value == error)
        );
        assert!(
            matches!(RoutingError::from(error.clone()), RoutingError::StorageFailed(value) if value == error)
        );
        assert!(
            matches!(RuntimeError::from(error.clone()), RuntimeError::StorageFailed(value) if value == error)
        );
    }
}

#[test]
fn adapter_error_keeps_original_code_fields_and_message() {
    let original = AppError {
        code: "adapter_specific".into(),
        message: "适配器错误".into(),
        fields: validation().fields,
        context: None,
    };
    assert!(
        matches!(ProxyError::from(original.clone()), ProxyError::OperationFailed(value) if value == original)
    );
    assert!(
        matches!(RoutingError::from(original.clone()), RoutingError::OperationFailed(value) if value == original)
    );
    assert!(
        matches!(RuntimeError::from(original.clone()), RuntimeError::OperationFailed(value) if value == original)
    );
    assert!(
        matches!(StorageError::from(original.clone()), StorageError::OperationFailed(value) if value == original)
    );
}

#[test]
fn runtime_errors_keep_transition_and_safe_display() {
    let transition = RuntimeError::InvalidTransition {
        from: RuntimePhase::Stopped,
        to: RuntimePhase::Running,
    };
    assert!(transition.to_string().contains("Stopped -> Running"));
    let source = AppError::unavailable("技术原因仅供诊断");
    let error = RuntimeError::SystemProxyFailed(source.clone());
    assert_eq!(error.to_string(), "系统代理设置失败");
    assert_eq!(
        error.source().expect("proxy source").to_string(),
        source.message
    );
    let credential = CredentialError::Unavailable(source);
    assert_eq!(credential.to_string(), "凭据存储不可用");
    assert!(credential.source().is_some());
}
