//! 运行时领域服务，保留模式持久化与网络恢复顺序。
use super::{context::ConfigurationContext, interfaces::RuntimeService as RuntimeServiceInterface};
use crate::{
    domain_errors::RuntimeError, error::AppError, models::RuntimeMode,
    observability::ActiveConnectionsSnapshot, runtime::RuntimeSnapshot,
};
use std::sync::{atomic::Ordering, Arc};

/// 协调运行时状态与配置恢复，底层仍使用 `RuntimeCoordinator`。
pub struct RuntimeService {
    context: Arc<ConfigurationContext>,
}
impl RuntimeService {
    pub(crate) fn new(context: Arc<ConfigurationContext>) -> Self {
        Self { context }
    }
}
impl RuntimeServiceInterface for RuntimeService {
    #[tracing::instrument(skip_all, level = "debug")]
    fn snapshot(&self) -> RuntimeSnapshot {
        self.context.runtime.snapshot()
    }
    #[tracing::instrument(skip_all, level = "debug")]
    fn active_connections(&self) -> ActiveConnectionsSnapshot {
        let mut snapshot = self.context.runtime.active_connections();
        if let Some(count) = snapshot.active_count {
            snapshot.trend = crate::store::diagnostic_now_ms()
                .and_then(|now| self.context.store.connection_trend(count, now))
                .unwrap_or_else(|error| {
                    tracing::warn!(code = %error.code, "连接趋势采样不可用");
                    None
                });
        }
        snapshot
    }
    #[tracing::instrument(skip_all, level = "debug")]
    fn set_mode(&self, mode: RuntimeMode) -> Result<RuntimeSnapshot, RuntimeError> {
        let _guard = self.context.mutation_lock()?;
        let current = self.context.store.load()?;
        let mut candidate = current.clone();
        candidate.runtime_mode = mode;
        self.context.commit(&current, &candidate)?;
        let snapshot = self.context.runtime.snapshot();
        if self.context.runtime.supports_proxy_runtime()
            && mode != RuntimeMode::Direct
            && snapshot.applied_mode != Some(mode)
        {
            self.context.runtime.request_mode(mode).map_err(Into::into)
        } else {
            Ok(snapshot)
        }
    }
    #[tracing::instrument(skip_all, level = "debug")]
    fn stop(&self) -> Result<RuntimeSnapshot, RuntimeError> {
        let _guard = self.context.mutation_lock()?;
        let current = self.context.store.load()?;
        let mut candidate = current.clone();
        candidate.runtime_mode = RuntimeMode::Direct;
        self.context.commit(&current, &candidate)?;
        self.context.runtime.stop().map_err(Into::into)
    }
    #[tracing::instrument(skip_all, level = "debug")]
    fn recover_network(&self) -> Result<RuntimeSnapshot, RuntimeError> {
        let _guard = self.context.lock()?;
        let recovery: Result<RuntimeSnapshot, AppError> = (|| {
            self.context.runtime.recover_network()?;
            crate::configuration_startup_recovery::recover_configuration_after_network_recovery(
                self.context.store.as_ref(),
                self.context.credentials.as_ref(),
                self.context.startup.as_ref(),
            )?;
            let current = self.context.store.load()?;
            let mut candidate = current.clone();
            candidate.runtime_mode = RuntimeMode::Direct;
            self.context.commit(&current, &candidate)?;
            Ok(self.context.runtime.snapshot())
        })();
        match &recovery {
            Ok(_) => self
                .context
                .startup_recovery_pending
                .store(false, Ordering::Relaxed),
            Err(error) => {
                self.context
                    .startup_recovery_pending
                    .store(true, Ordering::Relaxed);
                self.context.report_configuration_recovery_issue(error);
            }
        }
        recovery.map_err(Into::into)
    }
}

#[cfg(test)]
#[path = "runtime_service_tests.rs"]
mod tests;
