use super::*;
use crate::error_context::ErrorDomain;

#[test]
fn every_declared_domain_error_preserves_the_expected_root_context() {
    use ErrorDomain::*;
    let fields = || ValidationError { fields: vec![] };
    let source = || AppError::unavailable("适配器失败");
    let cases: Vec<(AppError, ErrorDomain, &str)> = vec![
        (fields().into(), Validation, "validation_failed"),
        (CredentialError::Missing.into(), Credential, "missing"),
        (CredentialError::Disabled.into(), Credential, "disabled"),
        (
            CredentialError::Invalid(fields()).into(),
            Validation,
            "validation_failed",
        ),
        (
            CredentialError::Unavailable(source()).into(),
            Credential,
            "unavailable",
        ),
        (StorageError::ReadFailed.into(), Storage, "read_failed"),
        (StorageError::WriteFailed.into(), Storage, "write_failed"),
        (
            StorageError::TransactionFailed.into(),
            Storage,
            "transaction_failed",
        ),
        (
            StorageError::CorruptedData.into(),
            Storage,
            "corrupted_data",
        ),
        (
            StorageError::UnsupportedVersion.into(),
            Storage,
            "unsupported_version",
        ),
        (
            StorageError::LockUnavailable.into(),
            Storage,
            "lock_unavailable",
        ),
        (
            StorageError::OperationFailed(source()).into(),
            Storage,
            "operation_failed",
        ),
        (
            ProxyError::NotFound {
                id: "missing".into(),
            }
            .into(),
            Proxy,
            "not_found",
        ),
        (
            ProxyError::InUse { references: vec![] }.into(),
            Proxy,
            "in_use",
        ),
        (
            ProxyError::CredentialInvalid(CredentialError::Missing).into(),
            Credential,
            "missing",
        ),
        (
            ProxyError::ValidationFailed(fields()).into(),
            Validation,
            "validation_failed",
        ),
        (
            ProxyError::StorageFailed(StorageError::WriteFailed).into(),
            Storage,
            "write_failed",
        ),
        (
            ProxyError::RecoveryInProgress.into(),
            Proxy,
            "recovery_in_progress",
        ),
        (
            ProxyError::OperationFailed(source()).into(),
            Proxy,
            "operation_failed",
        ),
        (
            RoutingError::RuleValidationFailed(fields()).into(),
            Validation,
            "validation_failed",
        ),
        (
            RoutingError::ChinaRulesUnavailable.into(),
            Routing,
            "china_rules_unavailable",
        ),
        (
            RoutingError::InvalidTarget(fields()).into(),
            Validation,
            "validation_failed",
        ),
        (
            RoutingError::StorageFailed(StorageError::ReadFailed).into(),
            Storage,
            "read_failed",
        ),
        (
            RoutingError::RecoveryInProgress.into(),
            Routing,
            "recovery_in_progress",
        ),
        (
            RoutingError::OperationFailed(source()).into(),
            Routing,
            "operation_failed",
        ),
        (
            RuntimeError::RecoveryInProgress.into(),
            Runtime,
            "recovery_in_progress",
        ),
        (
            RuntimeError::ProcessStartFailed {
                reason: "启动失败".into(),
            }
            .into(),
            Runtime,
            "process_start_failed",
        ),
        (
            RuntimeError::InvalidTransition {
                from: RuntimePhase::Stopped,
                to: RuntimePhase::Running,
            }
            .into(),
            Runtime,
            "invalid_transition",
        ),
        (
            RuntimeError::SystemProxyFailed(source()).into(),
            Runtime,
            "system_proxy_failed",
        ),
        (
            RuntimeError::InvariantViolated("内部详情".into()).into(),
            Runtime,
            "invariant_violated",
        ),
        (RuntimeError::Unavailable.into(), Runtime, "unavailable"),
        (
            RuntimeError::StorageFailed(StorageError::ReadFailed).into(),
            Storage,
            "read_failed",
        ),
        (
            RuntimeError::OperationFailed(source()).into(),
            Runtime,
            "operation_failed",
        ),
    ];
    for (error, domain, kind) in cases {
        let context = error.context.expect("typed error has context");
        assert_eq!(context.domain, domain);
        assert_eq!(context.kind, kind);
        assert!(!context.error_id.is_empty());
        assert!(!context.recovery_suggestion.is_empty());
    }
}
