use super::*;
use crate::latency_tasks::{
    LatencyInput, LatencyResult, LatencySubscription, LatencyTaskRegistry, LatencyTaskSnapshot,
    LatencyTaskState,
};
use std::{sync::Arc, time::Duration};

impl ConfigurationService {
    #[cfg(any(windows, test))]
    pub(crate) fn with_latency_tasks(self, registry: Arc<LatencyTaskRegistry>) -> Self {
        *self
            .latency_tasks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = Some(registry);
        self
    }
    fn latency_registry(&self) -> Result<Arc<LatencyTaskRegistry>, AppError> {
        self.latency_tasks
            .lock()
            .map_err(|_| AppError::storage("测速注册表不可用"))?
            .clone()
            .ok_or_else(|| AppError::unavailable("当前平台不支持代理延迟测试"))
    }
    pub(crate) fn start_latency_task(&self, id: &str) -> Result<LatencySubscription, AppError> {
        // Capture configuration, revision and the exact versioned secret under
        // the same writer lock; enqueue before permitting a new commit.
        let _guard = self.mutation_lock()?;
        let registry = self.latency_registry()?;
        registry.subscribe(self.latency_input(id)?)
    }
    fn latency_input(&self, id: &str) -> Result<LatencyInput, AppError> {
        let configuration = self.store.load()?;
        let profile = configuration
            .profiles
            .iter()
            .find(|profile| profile.id == id)
            .ok_or_else(|| AppError::unavailable("代理档案不存在"))?;
        if !profile.enabled {
            return Err(AppError::unavailable("已停用的代理不能测试"));
        }
        let credential =
            crate::credentials::read_profile_credential(self.credentials.as_ref(), profile)?;
        if profile.authentication_enabled && credential.is_none() {
            return Err(AppError::unavailable("代理认证凭据缺失"));
        }
        let probe = crate::models::PersistedConfiguration {
            profiles: vec![profile.clone()],
            active_profile_id: Some(id.to_owned()),
            settings: configuration.settings.clone(),
            ..Default::default()
        };
        Ok(LatencyInput {
            profile_id: id.to_owned(),
            configuration_revision: self.store.recovery_revision()?,
            configuration: probe,
            #[cfg(any(windows, test))]
            credential,
        })
    }
    pub(crate) fn latency_task_snapshot(
        &self,
        subscription_id: &str,
    ) -> Result<LatencyTaskSnapshot, AppError> {
        self.latency_registry()?.snapshot(subscription_id)
    }
    pub(crate) fn release_latency_task(&self, subscription_id: &str) -> Result<(), AppError> {
        self.latency_registry()?.release(subscription_id)
    }
    pub(crate) fn invalidate_latency_tasks(&self, revision: u64) {
        if let Ok(Some(registry)) = self.latency_tasks.lock().map(|registry| registry.clone()) {
            registry.invalidate(revision);
        }
    }
    pub(crate) fn test_latency_compat(&self, id: &str) -> Result<LatencyResult, AppError> {
        let registry = self.latency_registry()?;
        let subscription = self.start_latency_task(id)?;
        let result = (|| loop {
            let snapshot = registry.snapshot(&subscription.subscription_id)?;
            match snapshot.state {
                LatencyTaskState::Succeeded => {
                    return snapshot
                        .result
                        .ok_or_else(|| AppError::unavailable("测速结果缺失"))
                }
                LatencyTaskState::Failed => {
                    return Err(snapshot
                        .error
                        .unwrap_or_else(|| AppError::unavailable("测速失败")))
                }
                LatencyTaskState::Cancelled | LatencyTaskState::Cancelling => {
                    return Err(AppError::unavailable("测速已取消或输入版本已失效"))
                }
                _ => std::thread::sleep(Duration::from_millis(25)),
            }
        })();
        registry.release(&subscription.subscription_id)?;
        result
    }
}
