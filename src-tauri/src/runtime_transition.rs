use super::*;
use crate::runtime_plan::RuntimePlan;
use std::collections::HashMap;

impl ManagedRuntime {
    pub(super) fn transition(
        &self,
        candidate: PersistedConfiguration,
        mode: RuntimeMode,
        explicit_stop: bool,
        staged: bool,
    ) -> Result<RuntimeSnapshot, AppError> {
        let versions = self
            .state
            .lock()
            .map_err(|_| runtime_error())?
            .credential_versions
            .clone();
        self.transition_with_versions(candidate, mode, explicit_stop, staged, versions)
    }

    pub(super) fn transition_with_versions(
        &self,
        candidate: PersistedConfiguration,
        mode: RuntimeMode,
        explicit_stop: bool,
        staged: bool,
        versions: HashMap<String, String>,
    ) -> Result<RuntimeSnapshot, AppError> {
        let mut before = self.state.lock().map_err(|_| runtime_error())?.clone();
        if before.pending.is_some() || before.pending_settings.is_some() {
            return Err(AppError::storage("上一候选修订仍在提交中"));
        }
        before.begin_operation();
        let validation = RuntimePlan::build(&candidate, mode, &versions);
        if let Err(error) = validation {
            let mut state = self.state.lock().map_err(|_| runtime_error())?;
            *state = before;
            state.desired_mode = mode;
            state.phase = RuntimePhase::Failed;
            state.fail_operation(error.message.clone());
            self.publish(state.snapshot());
            return Err(error);
        }
        {
            let mut state = self.state.lock().map_err(|_| runtime_error())?;
            state.last_operation = before.last_operation.clone();
            state.desired_mode = mode;
            state.phase = if mode == RuntimeMode::Direct {
                RuntimePhase::Recovering
            } else if before.session.is_some() {
                RuntimePhase::Switching
            } else {
                RuntimePhase::Starting
            };
            state.last_error = None;
            self.publish(state.snapshot());
        }
        let revision = before.revision + 1;
        let runtime_plan_revision = before.runtime_plan_revision + 1;
        let result = self.backend.transition(
            before.session.as_ref(),
            &candidate,
            mode,
            runtime_plan_revision,
        );
        let mut state = self.state.lock().map_err(|_| runtime_error())?;
        match result {
            Ok(session) if mode != RuntimeMode::Direct && session.is_none() => {
                *state = before;
                state.phase = RuntimePhase::Failed;
                state.desired_mode = mode;
                state.fail_operation("内核没有返回运行会话".into());
                self.publish(state.snapshot());
                Err(runtime_error())
            }
            Ok(session) => {
                let started_at = if mode == RuntimeMode::Direct {
                    None
                } else {
                    before.started_at.or_else(|| Some(Instant::now()))
                };
                if staged {
                    state.pending = Some(PendingConfiguration {
                        configuration: candidate,
                        revision,
                        mode,
                        explicit_stop,
                        session,
                        started_at,
                        credential_versions: versions,
                        runtime_plan_revision,
                    });
                } else {
                    state.configuration = candidate;
                    state.revision = revision;
                    state.runtime_plan_revision = runtime_plan_revision;
                    state.credential_versions = versions;
                    state.applied_mode = if explicit_stop { None } else { Some(mode) };
                    state.phase = if mode == RuntimeMode::Direct {
                        RuntimePhase::Stopped
                    } else {
                        RuntimePhase::Running
                    };
                    state.started_at = started_at;
                    state.session = session;
                    state.session_health = if state.session.is_some() {
                        SessionHealth::Healthy
                    } else {
                        SessionHealth::Inactive
                    };
                    state.complete_operation();
                    self.backend.confirm_transition(before.session.as_ref());
                    self.publish(state.snapshot());
                }
                Ok(state.snapshot())
            }
            Err(error) => {
                *state = before;
                state.desired_mode = mode;
                state.phase = RuntimePhase::Failed;
                state.fail_operation(error.message.clone());
                self.publish(state.snapshot());
                Err(error)
            }
        }
    }
}
