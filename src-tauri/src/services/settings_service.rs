//! 应用设置服务，事务性协调配置和系统启动项。
use super::context::ConfigurationContext;
use crate::{error::AppError, models::AppSettings};
use std::sync::Arc;
pub(crate) struct SettingsService {
    context: Arc<ConfigurationContext>,
}
impl SettingsService {
    pub(crate) fn new(context: Arc<ConfigurationContext>) -> Self {
        Self { context }
    }
    pub(crate) fn settings(&self) -> Result<AppSettings, AppError> {
        let mut settings = self.context.store.load()?.settings;
        settings.launch_at_login = self.context.startup.is_enabled()?;
        Ok(settings)
    }
    pub(crate) fn update_settings(&self, settings: AppSettings) -> Result<AppSettings, AppError> {
        let _guard = self.context.mutation_lock()?;
        let current = self.context.store.load()?;
        let startup = self.context.startup_change(settings.launch_at_login)?;
        let mut candidate = current.clone();
        candidate.settings = settings;
        self.context
            .commit_effects(&current, &candidate, vec![], startup)?;
        self.settings()
    }
}
