use super::*;
use crate::error_context::ErrorDomain;

fn error(code: &str, message: impl Into<String>, fields: Vec<FieldError>) -> AppError {
    AppError {
        code: code.into(),
        message: message.into(),
        fields,
        context: None,
    }
}

impl From<ValidationError> for AppError {
    fn from(value: ValidationError) -> Self {
        Self::validation(value.fields).with_context(ErrorDomain::Validation, "validation_failed")
    }
}

impl From<CredentialError> for AppError {
    fn from(value: CredentialError) -> Self {
        let kind = match &value {
            CredentialError::Missing => "missing",
            CredentialError::Disabled => "disabled",
            CredentialError::Invalid(_) => "invalid",
            CredentialError::Unavailable(_) => "unavailable",
        };
        let converted: AppError = match value {
            CredentialError::Invalid(validation) => validation.into(),
            CredentialError::Unavailable(source) => source,
            other => error("credential_error", other.to_string(), vec![]),
        };
        converted.with_context(ErrorDomain::Credential, kind)
    }
}

impl From<StorageError> for AppError {
    fn from(value: StorageError) -> Self {
        let kind = match &value {
            StorageError::ReadFailed => "read_failed",
            StorageError::WriteFailed => "write_failed",
            StorageError::TransactionFailed => "transaction_failed",
            StorageError::CorruptedData => "corrupted_data",
            StorageError::UnsupportedVersion => "unsupported_version",
            StorageError::LockUnavailable => "lock_unavailable",
            StorageError::OperationFailed(_) => "operation_failed",
        };
        let converted: AppError = match value {
            StorageError::OperationFailed(source) => source,
            other => Self::storage(other.to_string()),
        };
        converted.with_context(ErrorDomain::Storage, kind)
    }
}

impl From<ProxyError> for AppError {
    fn from(value: ProxyError) -> Self {
        let kind = match &value {
            ProxyError::NotFound { .. } => "not_found",
            ProxyError::InUse { .. } => "in_use",
            ProxyError::CredentialInvalid(_) => "credential_invalid",
            ProxyError::ValidationFailed(_) => "validation_failed",
            ProxyError::StorageFailed(_) => "storage_failed",
            ProxyError::RecoveryInProgress => "recovery_in_progress",
            ProxyError::OperationFailed(_) => "operation_failed",
        };
        let converted: AppError = match value {
            ProxyError::NotFound { id } => {
                error("proxy_not_found", format!("代理档案不存在: {id}"), vec![])
            }
            ProxyError::InUse { references } => error(
                "proxy_in_use",
                "代理正在被使用，无法删除",
                references
                    .into_iter()
                    .map(|reference| FieldError {
                        field: reference.field,
                        message: reference.description,
                    })
                    .collect(),
            ),
            ProxyError::CredentialInvalid(source) => source.into(),
            ProxyError::ValidationFailed(source) => source.into(),
            ProxyError::StorageFailed(source) => source.into(),
            ProxyError::RecoveryInProgress => crate::configuration_recovery::recovery_error(),
            ProxyError::OperationFailed(source) => source,
        };
        converted.with_context(ErrorDomain::Proxy, kind)
    }
}

impl From<RoutingError> for AppError {
    fn from(value: RoutingError) -> Self {
        let kind = match &value {
            RoutingError::RuleValidationFailed(_) => "rule_validation_failed",
            RoutingError::ChinaRulesUnavailable => "china_rules_unavailable",
            RoutingError::InvalidTarget(_) => "invalid_target",
            RoutingError::StorageFailed(_) => "storage_failed",
            RoutingError::RecoveryInProgress => "recovery_in_progress",
            RoutingError::OperationFailed(_) => "operation_failed",
        };
        let converted: AppError = match value {
            RoutingError::RuleValidationFailed(source) | RoutingError::InvalidTarget(source) => {
                source.into()
            }
            RoutingError::StorageFailed(source) => source.into(),
            RoutingError::ChinaRulesUnavailable => {
                error("china_rules_unavailable", value.to_string(), vec![])
            }
            RoutingError::RecoveryInProgress => crate::configuration_recovery::recovery_error(),
            RoutingError::OperationFailed(source) => source,
        };
        converted.with_context(ErrorDomain::Routing, kind)
    }
}

impl From<RuntimeError> for AppError {
    fn from(value: RuntimeError) -> Self {
        let kind = match &value {
            RuntimeError::RecoveryInProgress => "recovery_in_progress",
            RuntimeError::ProcessStartFailed { .. } => "process_start_failed",
            RuntimeError::InvalidTransition { .. } => "invalid_transition",
            RuntimeError::SystemProxyFailed(_) => "system_proxy_failed",
            RuntimeError::InvariantViolated(_) => "invariant_violated",
            RuntimeError::Unavailable => "unavailable",
            RuntimeError::StorageFailed(_) => "storage_failed",
            RuntimeError::OperationFailed(_) => "operation_failed",
        };
        let converted: AppError = match value {
            RuntimeError::RecoveryInProgress => crate::configuration_recovery::recovery_error(),
            RuntimeError::ProcessStartFailed { reason } => error(
                "process_start_failed",
                format!("进程启动失败: {reason}"),
                vec![],
            ),
            RuntimeError::InvalidTransition { from, to } => error(
                "invalid_transition",
                format!("无效的状态转换: {from:?} -> {to:?}"),
                vec![],
            ),
            RuntimeError::SystemProxyFailed(_) => {
                error("system_proxy_failed", "系统代理设置失败", vec![])
            }
            RuntimeError::InvariantViolated(_) => error(
                "runtime_invariant_violated",
                "运行时状态异常，请停止代理并执行网络恢复",
                vec![],
            ),
            RuntimeError::Unavailable => Self::unavailable("当前平台不支持此运行时操作"),
            RuntimeError::StorageFailed(source) => source.into(),
            RuntimeError::OperationFailed(source) => source,
        };
        converted.with_context(ErrorDomain::Runtime, kind)
    }
}
