use crate::{
    error::AppError,
    models::{PersistedConfiguration, RuntimeMode},
    runtime::{BackendSession, RuntimeBackend},
};

/// Temporary fail-closed boundary until the managed core and Windows proxy
/// adapter can commit as a single runtime transition.
pub struct UnavailableRuntimeBackend;

impl RuntimeBackend for UnavailableRuntimeBackend {
    fn transition(
        &self,
        _: Option<&BackendSession>,
        _: &PersistedConfiguration,
        mode: RuntimeMode,
        _: u64,
    ) -> Result<Option<BackendSession>, AppError> {
        if mode == RuntimeMode::Direct {
            Ok(None)
        } else {
            Err(AppError::unavailable("当前平台不支持代理内核与系统代理"))
        }
    }

    fn confirm_transition(&self, _: Option<&BackendSession>) {}

    fn revert_transition(
        &self,
        _: Option<&BackendSession>,
        _: Option<&BackendSession>,
    ) -> Result<(), AppError> {
        Ok(())
    }

    fn reconcile_session(&self, _: &BackendSession) -> Result<bool, AppError> {
        Ok(false)
    }
}
