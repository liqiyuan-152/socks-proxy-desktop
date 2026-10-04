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
        let operation = if mode == RuntimeMode::Direct {
            "runtime_stop"
        } else if self
            .state
            .lock()
            .is_ok_and(|state| state.session().is_some())
        {
            "runtime_switch"
        } else {
            "runtime_start"
        };
        crate::performance_metrics::measure(operation, || {
            let mut before = self.state.lock().map_err(|_| runtime_error())?.clone();
            if before.pending.is_some() || before.pending_settings.is_some() {
                return Err(AppError::storage("上一候选修订仍在提交中"));
            }
            before.begin_operation();
            let validation = RuntimePlan::build(&candidate, mode, &versions);
            if let Err(error) = validation {
                let mut state = self.state.lock().map_err(|_| runtime_error())?;
                before.history = state.history.clone();
                *state = before;
                state.desired_mode = mode;
                state.apply_event(RuntimeEvent::OperationFailed {
                    error: error.message.clone(),
                })?;
                state.fail_operation(error.message.clone());
                self.publish(state.snapshot());
                return Err(error);
            }
            {
                let mut state = self.state.lock().map_err(|_| runtime_error())?;
                state.last_operation = before.last_operation.clone();
                state.desired_mode = mode;
                let event = if explicit_stop {
                    RuntimeEvent::StopRequested
                } else if before.session().is_some() {
                    RuntimeEvent::ModeSwitchRequested { mode }
                } else {
                    RuntimeEvent::StartRequested { mode }
                };
                state.apply_event(event)?;
                state.last_error = None;
                self.publish(state.snapshot());
            }
            let revision = before.revision + 1;
            let runtime_plan_revision = before.runtime_plan_revision + 1;
            let result =
                self.backend
                    .transition(before.session(), &candidate, mode, runtime_plan_revision);
            let mut state = self.state.lock().map_err(|_| runtime_error())?;
            match result {
                Ok(session) if mode != RuntimeMode::Direct && session.is_none() => {
                    before.history = state.history.clone();
                    *state = before;
                    state.apply_event(RuntimeEvent::OperationFailed {
                        error: "内核没有返回运行会话".into(),
                    })?;
                    state.desired_mode = mode;
                    state.fail_operation("内核没有返回运行会话".into());
                    self.publish(state.snapshot());
                    Err(runtime_error())
                }
                Ok(session) => {
                    let started_at = if mode == RuntimeMode::Direct {
                        None
                    } else {
                        Some(
                            before
                                .node
                                .active()
                                .map_or_else(Instant::now, |active| active.started_at),
                        )
                    };
                    if let Some(session) = session.as_ref() {
                        let prepared = state.apply_event(RuntimeEvent::ProcessStarted {
                            active: ActiveSession {
                                session: session.clone(),
                                mode,
                                started_at: started_at.unwrap_or_else(Instant::now),
                            },
                        });
                        if let Err(error) = prepared {
                            let reverted = self
                                .backend
                                .revert_transition(Some(session), before.session());
                            before.history = state.history.clone();
                            *state = before;
                            state.desired_mode = mode;
                            let failure = match reverted {
                                Ok(()) => {
                                    state.apply_event(RuntimeEvent::OperationFailed {
                                        error: error.message.clone(),
                                    })?;
                                    error
                                }
                                Err(recovery) => {
                                    let event = if state.session().is_some() {
                                        RuntimeEvent::ProcessExited {
                                            recovery_error: Some(recovery.message.clone()),
                                        }
                                    } else {
                                        RuntimeEvent::RecoveryBlocked {
                                            error: recovery.message.clone(),
                                        }
                                    };
                                    state.apply_event(event)?;
                                    recovery
                                }
                            };
                            state.fail_operation(failure.message.clone());
                            self.publish(state.snapshot());
                            return Err(failure);
                        }
                    }
                    if staged {
                        state.pending = Some(PendingConfiguration {
                            configuration: candidate,
                            revision,
                            mode,
                            explicit_stop,
                            session,
                            previous_node: before.node.clone(),
                            credential_versions: versions,
                            runtime_plan_revision,
                        });
                    } else {
                        state.configuration = candidate;
                        state.revision = revision;
                        state.runtime_plan_revision = runtime_plan_revision;
                        state.credential_versions = versions;
                        state.commit_session(explicit_stop)?;
                        state.complete_operation();
                        self.backend.confirm_transition(before.session());
                        self.publish(state.snapshot());
                    }
                    Ok(state.snapshot())
                }
                Err(error) => {
                    before.history = state.history.clone();
                    *state = before;
                    state.desired_mode = mode;
                    state.apply_event(RuntimeEvent::OperationFailed {
                        error: error.message.clone(),
                    })?;
                    state.fail_operation(error.message.clone());
                    self.publish(state.snapshot());
                    Err(error)
                }
            }
        })
    }
}
