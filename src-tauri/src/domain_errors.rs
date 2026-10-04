//! 领域错误边界；IPC 仍由应用 facade 转换为既有 `AppError`。
//!
//! 变体只携带业务标识和可安全展示的上下文，不携带密码、数据库路径
//! 或 SQL。适配器尚未迁移的错误通过 `OperationFailed` 保留原始契约。

use crate::{error::AppError, runtime::RuntimePhase};
use thiserror::Error;

pub use crate::error::FieldError;

mod adapters;
mod conversions;

/// 指明阻止档案删除的配置位置。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceLocation {
    /// 与前端表单字段一致的位置，例如 `rules[0].proxy_profile_id`。
    pub field: String,
    /// 不含认证信息的引用说明。
    pub description: String,
}

/// 保留所有字段问题，避免只显示第一个验证失败。
#[derive(Clone, Debug, Eq, PartialEq, Error)]
#[error("配置包含无效字段")]
pub struct ValidationError {
    /// 校验失败的字段及可展示的原因。
    pub fields: Vec<FieldError>,
}

/// 凭据错误只描述状态，不包含凭据内容。
#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum CredentialError {
    #[error("认证凭据缺失，请重新配置")]
    Missing,
    #[error("此代理未启用认证")]
    Disabled,
    #[error("凭据验证失败")]
    Invalid(#[from] ValidationError),
    #[error("凭据存储不可用")]
    Unavailable(#[source] AppError),
}

/// 细化持久化失败的位置；技术原因应写入经过脱敏的诊断记录。
#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum StorageError {
    #[error("本地配置数据库读取失败")]
    ReadFailed,
    #[error("本地配置数据库写入失败")]
    WriteFailed,
    #[error("本地配置事务提交失败")]
    TransactionFailed,
    #[error("已保存的配置无法读取")]
    CorruptedData,
    #[error("数据库版本高于当前应用支持的版本")]
    UnsupportedVersion,
    #[error("本地配置存储锁不可用")]
    LockUnavailable,
    #[error(transparent)]
    OperationFailed(#[from] AppError),
}

/// 代理档案服务可以返回的业务失败。
#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum ProxyError {
    #[error("代理档案不存在: {id}")]
    NotFound { id: String },
    #[error("代理正在被使用，无法删除")]
    InUse { references: Vec<ReferenceLocation> },
    #[error("凭据验证失败")]
    CredentialInvalid(#[from] CredentialError),
    #[error("代理档案验证失败")]
    ValidationFailed(#[from] ValidationError),
    #[error("存储操作失败: {0}")]
    StorageFailed(#[from] StorageError),
    #[error("运行时正在恢复中，操作被拒绝")]
    RecoveryInProgress,
    #[error(transparent)]
    OperationFailed(AppError),
}

/// 路由服务可以返回的业务失败。
#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum RoutingError {
    #[error("路由规则验证失败")]
    RuleValidationFailed(#[from] ValidationError),
    #[error("国内直连规则集缺失、损坏或与内核版本不兼容")]
    ChinaRulesUnavailable,
    #[error("路由测试目标无效")]
    InvalidTarget(ValidationError),
    #[error("存储操作失败: {0}")]
    StorageFailed(#[from] StorageError),
    #[error("运行时正在恢复中，操作被拒绝")]
    RecoveryInProgress,
    #[error(transparent)]
    OperationFailed(AppError),
}

/// 运行时协调和状态转换中的可识别失败。
#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum RuntimeError {
    #[error("运行时正在恢复中，操作被拒绝")]
    RecoveryInProgress,
    #[error("进程启动失败: {reason}")]
    ProcessStartFailed { reason: String },
    #[error("无效的状态转换: {from:?} -> {to:?}")]
    InvalidTransition {
        from: RuntimePhase,
        to: RuntimePhase,
    },
    #[error("系统代理设置失败")]
    SystemProxyFailed(#[source] AppError),
    #[error("运行时状态不满足约束: {0}")]
    InvariantViolated(String),
    #[error("当前平台不支持此运行时操作")]
    Unavailable,
    #[error("存储操作失败: {0}")]
    StorageFailed(#[from] StorageError),
    #[error(transparent)]
    OperationFailed(AppError),
}

#[cfg(test)]
#[path = "domain_errors_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "domain_error_conversion_tests.rs"]
mod conversion_tests;

#[cfg(test)]
#[path = "domain_error_adapter_tests.rs"]
mod adapter_tests;

#[cfg(test)]
#[path = "domain_error_context_matrix_tests.rs"]
mod context_matrix_tests;
