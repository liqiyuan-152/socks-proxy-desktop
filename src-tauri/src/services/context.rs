//! 各服务共享的持久化事务和恢复边界；由应用 facade 持有。
use crate::{
    credentials::CredentialStore, error::AppError, runtime::RuntimeCoordinator,
    startup::StartupAdapter, store::ConfigurationStore,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

pub(crate) struct ConfigurationContext {
    pub(crate) store: Arc<dyn ConfigurationStore>,
    pub(crate) credentials: Arc<dyn CredentialStore>,
    pub(crate) startup: Arc<dyn StartupAdapter>,
    pub(crate) runtime: Arc<dyn RuntimeCoordinator>,
    pub(crate) china_rule_root: Mutex<Option<PathBuf>>,
    pub(crate) serial: Mutex<()>,
    pub(crate) startup_recovery_pending: AtomicBool,
    pub(crate) latency_tasks:
        Mutex<Option<std::sync::Arc<crate::latency_tasks::LatencyTaskRegistry>>>,
}

impl ConfigurationContext {
    pub fn new(
        store: Arc<dyn ConfigurationStore>,
        credentials: Arc<dyn CredentialStore>,
        startup: Arc<dyn StartupAdapter>,
        runtime: Arc<dyn RuntimeCoordinator>,
    ) -> Self {
        Self {
            store,
            credentials,
            startup,
            runtime,
            china_rule_root: Mutex::new(None),
            serial: Mutex::new(()),
            startup_recovery_pending: AtomicBool::new(false),
            latency_tasks: Mutex::new(None),
        }
    }

    pub(crate) fn mutation_lock(&self) -> Result<std::sync::MutexGuard<'_, ()>, AppError> {
        let guard = self.lock()?;
        if self.startup_recovery_pending.load(Ordering::Relaxed)
            || self.store.recovery_record()?.is_some()
        {
            return Err(crate::configuration_recovery::recovery_error());
        }
        Ok(guard)
    }

    pub(crate) fn lock(&self) -> Result<std::sync::MutexGuard<'_, ()>, AppError> {
        self.serial
            .lock()
            .map_err(|_| AppError::storage("配置写入锁不可用"))
    }
}

fn rollback_failed() -> AppError {
    AppError {
        code: "rollback_failed".into(),
        message: "恢复上次配置失败，需要检查运行时与系统代理状态".into(),
        fields: Vec::new(),
        context: None,
    }
}

#[path = "../configuration_transaction.rs"]
mod transaction;

#[path = "../configuration_latency.rs"]
mod latency;
