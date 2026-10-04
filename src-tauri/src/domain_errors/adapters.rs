//! 旧适配器的稳定错误码映射到领域类型，不通过消息文字猜测根因。
use super::*;

impl From<AppError> for ProxyError {
    fn from(error: AppError) -> Self {
        if error.context.is_some() {
            return Self::OperationFailed(error);
        }
        match error.code.as_str() {
            "validation_error" => Self::ValidationFailed(ValidationError {
                fields: error.fields,
            }),
            "credential_error" => Self::CredentialInvalid(CredentialError::Unavailable(error)),
            "storage_error" => Self::StorageFailed(StorageError::OperationFailed(error)),
            "configuration_recovery" => Self::RecoveryInProgress,
            _ => Self::OperationFailed(error),
        }
    }
}

impl From<AppError> for RoutingError {
    fn from(error: AppError) -> Self {
        if error.context.is_some() {
            return Self::OperationFailed(error);
        }
        match error.code.as_str() {
            "validation_error" => Self::RuleValidationFailed(ValidationError {
                fields: error.fields,
            }),
            "china_rules_unavailable" => Self::ChinaRulesUnavailable,
            "storage_error" => Self::StorageFailed(StorageError::OperationFailed(error)),
            "configuration_recovery" => Self::RecoveryInProgress,
            _ => Self::OperationFailed(error),
        }
    }
}

impl From<AppError> for RuntimeError {
    fn from(error: AppError) -> Self {
        if error.context.is_some() {
            return Self::OperationFailed(error);
        }
        match error.code.as_str() {
            "storage_error" => Self::StorageFailed(StorageError::OperationFailed(error)),
            "configuration_recovery" => Self::RecoveryInProgress,
            "system_proxy_failed" => Self::SystemProxyFailed(error),
            _ => Self::OperationFailed(error),
        }
    }
}
