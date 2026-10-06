//! 应用健康检查与诊断；不改变现有 IPC 的错误、分页和恢复响应。
use super::application_service::ApplicationService;
use crate::{
    error::AppError,
    error_context::ErrorDomain,
    models::RuntimeMode,
    runtime::{RuntimeSnapshot, SessionHealth},
    store::{DiagnosticFilter, DiagnosticGroup, DiagnosticPage, RuntimeDiagnostic},
};
use serde::Serialize;
use std::sync::atomic::Ordering;

/// 一致配置版本上的服务健康状态，适用于内部诊断和测试。
#[derive(Debug)]
pub struct ServiceHealth {
    /// 当前已提交配置的版本号。
    pub configuration_revision: u64,
    /// 存在未完成的配置或网络恢复时为真。
    pub recovery_pending: bool,
    /// 运行时状态；健康检查不会主动启动或切换代理。
    pub runtime: RuntimeSnapshot,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
/// 保持现有 IPC 兼容的网络恢复响应。
pub struct NetworkRecoveryResult {
    pub completed_at_ms: i64,
    pub snapshot: RuntimeSnapshot,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
/// 明确声明不可验证的连接历史能力不可用。
pub struct HistoryUnavailable {
    pub available: bool,
    pub reason: &'static str,
}

impl ApplicationService {
    /// 验证配置可读取且有效，并报告恢复阻塞与运行时健康状态。
    pub fn health_check(&self) -> Result<ServiceHealth, AppError> {
        let _guard = self.context.lock()?;
        self.context.store.load()?.validate()?;
        let runtime = self.runtime_snapshot();
        let recovery_pending = self
            .context
            .startup_recovery_pending
            .load(Ordering::Relaxed)
            || self.context.store.recovery_record()?.is_some()
            || runtime.session_health == SessionHealth::RecoveryRequired;
        Ok(ServiceHealth {
            configuration_revision: self.context.store.recovery_revision()?,
            recovery_pending,
            runtime,
        })
    }

    /// 保持现有诊断过滤、分页与保留策略。
    pub fn runtime_diagnostics(
        &self,
        filter: &DiagnosticFilter,
        offset: usize,
        limit: usize,
    ) -> Result<DiagnosticPage, AppError> {
        self.finish(
            self.context.store.list_diagnostics(filter, offset, limit),
            ErrorDomain::Application,
            "get_runtime_diagnostics",
        )
    }

    /// 聚合匹配的领域错误；保留各次发生的独立错误编号。
    pub fn diagnostic_groups(
        &self,
        filter: &DiagnosticFilter,
    ) -> Result<Vec<DiagnosticGroup>, AppError> {
        self.finish(
            self.context.store.diagnostic_groups(filter),
            ErrorDomain::Application,
            "get_diagnostic_groups",
        )
    }

    /// 返回匹配诊断的完整 JSON Lines 快照。
    pub fn export_runtime_diagnostics(
        &self,
        filter: &DiagnosticFilter,
    ) -> Result<String, AppError> {
        self.finish(
            self.context.store.export_diagnostics(filter),
            ErrorDomain::Application,
            "export_runtime_diagnostics",
        )
    }

    /// 委托诊断存储执行确认检查及清理。
    pub fn clear_runtime_diagnostics(
        &self,
        filter: &DiagnosticFilter,
        confirmed: bool,
    ) -> Result<usize, AppError> {
        self.finish(
            self.context.store.clear_diagnostics(filter, confirmed),
            ErrorDomain::Application,
            "clear_runtime_diagnostics",
        )
    }

    /// 现有内核不提供可验证的连接历史，保持原有能力声明。
    pub fn connection_history(&self) -> HistoryUnavailable {
        HistoryUnavailable {
            available: false,
            reason: "当前内核不提供可验证的已完成连接结果或历史记录",
        }
    }

    pub(crate) fn record_mode_failure(&self, mode: RuntimeMode, error: &AppError) {
        let label = match mode {
            RuntimeMode::Rules { .. } => "规则代理",
            RuntimeMode::Global => "全局代理",
            RuntimeMode::Direct => "全局直连",
        };
        self.record_error(error, &format!("切换至{label}失败"));
    }

    /// 用户确认后恢复网络，并记录现有格式的恢复诊断。
    pub fn recover_network_confirmed(
        &self,
        confirmed: bool,
    ) -> Result<NetworkRecoveryResult, AppError> {
        if !confirmed {
            return self.finish(
                Err(AppError::unavailable("请先确认恢复本应用管理的系统代理")),
                ErrorDomain::Runtime,
                "recover_network",
            );
        }
        let result = self.recover_network();
        // Failure has already been annotated and recorded by recover_network.
        // Do not create a second identity for the same failed operation.
        let snapshot = result?;
        let completed_at_ms = self.finish(
            crate::store::diagnostic_now_ms(),
            ErrorDomain::Storage,
            "recover_network",
        )?;
        let recorded = self.context.store.record_diagnostic(&RuntimeDiagnostic {
            error_type: None,
            operation: None,
            id: uuid::Uuid::new_v4().to_string(),
            created_at_ms: completed_at_ms,
            severity: "info".into(),
            summary: "用户确认后已检查并恢复本应用可确认拥有的系统代理设置".into(),
        });
        self.finish(recorded, ErrorDomain::Storage, "recover_network")?;
        Ok(NetworkRecoveryResult {
            completed_at_ms,
            snapshot,
        })
    }
}
