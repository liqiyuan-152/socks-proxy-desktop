use super::*;
use crate::runtime_plan::{candidate_versions, RuntimePlan};

impl ManagedRuntime {
    pub(super) fn stage_configuration(
        &self,
        previous: &PersistedConfiguration,
        candidate: &PersistedConfiguration,
        changed_credentials: &[String],
    ) -> Result<RuntimeSnapshot, AppError> {
        let _guard = self.serialize_operation()?;
        let state = self.state.lock().map_err(|_| runtime_error())?.clone();
        if state.configuration != *previous {
            return Err(AppError::storage("配置修订与运行时不一致"));
        }
        if state.pending.is_some() || state.pending_settings.is_some() {
            return Err(AppError::storage("上一候选修订仍在提交中"));
        }
        let mode_changed = previous.runtime_mode != candidate.runtime_mode;
        let old_mode = state.node.applied_mode().unwrap_or(RuntimeMode::Direct);
        let mode = if !self.backend.supports_proxy_runtime() {
            RuntimeMode::Direct
        } else if mode_changed {
            candidate.runtime_mode
        } else {
            old_mode
        };
        let versions =
            candidate_versions(candidate, &state.credential_versions, changed_credentials);
        let old_plan = RuntimePlan::build(previous, old_mode, &state.credential_versions)?;
        let new_plan = RuntimePlan::build(candidate, mode, &versions)?;
        if old_plan == new_plan {
            let mut state = self.state.lock().map_err(|_| runtime_error())?;
            state.begin_operation();
            state.pending_settings = Some(PendingMetadata {
                configuration: candidate.clone(),
                credential_versions: versions,
            });
            self.publish(state.snapshot());
            return Ok(state.snapshot());
        }
        self.transition_with_versions(
            candidate.clone(),
            mode,
            !mode_changed && state.node.applied_mode().is_none(),
            true,
            versions,
        )
    }
}
