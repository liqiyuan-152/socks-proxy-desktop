use super::*;
use crate::error_context::ErrorDomain;

#[test]
fn known_adapter_codes_select_typed_variants_without_message_matching() {
    let validation = AppError::validation(vec![FieldError {
        field: "port".into(),
        message: "invalid".into(),
    }]);
    assert!(
        matches!(ProxyError::from(validation.clone()), ProxyError::ValidationFailed(value) if value.fields == validation.fields)
    );
    assert!(
        matches!(RoutingError::from(validation.clone()), RoutingError::RuleValidationFailed(value) if value.fields == validation.fields)
    );
    let storage = AppError::storage("any adapter-specific explanation");
    assert!(
        matches!(ProxyError::from(storage.clone()), ProxyError::StorageFailed(StorageError::OperationFailed(source)) if source == storage)
    );
    assert!(
        matches!(RoutingError::from(storage.clone()), RoutingError::StorageFailed(StorageError::OperationFailed(source)) if source == storage)
    );
    assert!(
        matches!(RuntimeError::from(storage.clone()), RuntimeError::StorageFailed(StorageError::OperationFailed(source)) if source == storage)
    );
    let recovery = crate::configuration_recovery::recovery_error();
    assert!(matches!(
        ProxyError::from(recovery.clone()),
        ProxyError::RecoveryInProgress
    ));
    assert!(matches!(
        RoutingError::from(recovery.clone()),
        RoutingError::RecoveryInProgress
    ));
    assert!(matches!(
        RuntimeError::from(recovery),
        RuntimeError::RecoveryInProgress
    ));
}

#[test]
fn nested_existing_context_keeps_the_original_identity_in_all_adapters() {
    let original =
        AppError::storage("internal failure").with_context(ErrorDomain::Storage, "write_failed");
    for converted in [
        AppError::from(ProxyError::from(original.clone())),
        AppError::from(RoutingError::from(original.clone())),
        AppError::from(RuntimeError::from(original.clone())),
    ] {
        assert_eq!(converted, original);
    }
}

#[test]
fn credential_and_system_proxy_codes_keep_sources_while_asset_code_has_its_own_variant() {
    let credential = AppError {
        code: "credential_error".into(),
        message: "credential unavailable".into(),
        fields: vec![],
        context: None,
    };
    assert!(
        matches!(ProxyError::from(credential.clone()), ProxyError::CredentialInvalid(CredentialError::Unavailable(source)) if source == credential)
    );
    let proxy = AppError {
        code: "system_proxy_failed".into(),
        message: "system setting failed".into(),
        fields: vec![],
        context: None,
    };
    assert!(
        matches!(RuntimeError::from(proxy.clone()), RuntimeError::SystemProxyFailed(source) if source == proxy)
    );
    let assets = AppError {
        code: "china_rules_unavailable".into(),
        message: "missing assets".into(),
        fields: vec![],
        context: None,
    };
    assert!(matches!(
        RoutingError::from(assets),
        RoutingError::ChinaRulesUnavailable
    ));
}
