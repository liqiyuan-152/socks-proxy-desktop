use super::*;
#[cfg(test)]
use crate::runtime_session::RUNTIME_SESSION_KEY;
use crate::{error::FieldError, runtime_session::SessionLease};
use std::{sync::Mutex, time::Duration};

pub struct ManagedRuntime {
    state: Mutex<RuntimeState>,
    operation: Mutex<()>,
    events: Mutex<Option<tokio::sync::mpsc::UnboundedSender<RuntimeSnapshot>>>,
    backend: Box<dyn RuntimeBackend>,
    _ownership: SessionLease,
}

impl ManagedRuntime {
    #[cfg(test)]
    pub fn new(
        configuration: PersistedConfiguration,
        backend: Box<dyn RuntimeBackend>,
    ) -> Result<Self, AppError> {
        let ownership = SessionLease::acquire(RUNTIME_SESSION_KEY)?;
        Self::from_lease(configuration, backend, ownership)
    }

    pub(crate) fn from_lease(
        configuration: PersistedConfiguration,
        backend: Box<dyn RuntimeBackend>,
        ownership: SessionLease,
    ) -> Result<Self, AppError> {
        configuration.validate()?;
        Ok(Self {
            state: Mutex::new(RuntimeState {
                configuration,
                revision: 0,
                desired_mode: RuntimeMode::Direct,
                applied_mode: None,
                phase: RuntimePhase::Stopped,
                session: None,
                started_at: None,
                last_error: None,
                pending: None,
            }),
            operation: Mutex::new(()),
            events: Mutex::new(None),
            backend,
            _ownership: ownership,
        })
    }

    pub fn subscribe(&self) -> tokio::sync::mpsc::UnboundedReceiver<RuntimeSnapshot> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        *self
            .events
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = Some(sender);
        receiver
    }

    fn publish(&self, snapshot: RuntimeSnapshot) {
        if let Ok(events) = self.events.lock() {
            if let Some(sender) = events.as_ref() {
                let _ = sender.send(snapshot);
            }
        }
    }

    /// Surface an uncompleted startup recovery without preventing access to
    /// configuration or discarding the durable ownership journal.
    pub(crate) fn report_startup_recovery_issue(&self, message: String) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        state.phase = RuntimePhase::Failed;
        state.last_error = Some(message);
        self.publish(state.snapshot());
    }

    fn transition(
        &self,
        candidate: PersistedConfiguration,
        mode: RuntimeMode,
        explicit_stop: bool,
        staged: bool,
    ) -> Result<RuntimeSnapshot, AppError> {
        candidate.validate()?;
        if mode != RuntimeMode::Direct && candidate.active_profile_id.is_none() {
            return Err(AppError::validation(vec![FieldError {
                field: "active_profile_id".into(),
                message: "代理模式需要选择活动档案".into(),
            }]));
        }
        let before = self.state.lock().map_err(|_| runtime_error())?.clone();
        if before.pending.is_some() {
            return Err(AppError::storage("上一候选修订仍在提交中"));
        }
        {
            let mut state = self.state.lock().map_err(|_| runtime_error())?;
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
        let result = self
            .backend
            .transition(before.session.as_ref(), &candidate, mode, revision);
        let mut state = self.state.lock().map_err(|_| runtime_error())?;
        match result {
            Ok(session) if mode != RuntimeMode::Direct && session.is_none() => {
                *state = before;
                state.phase = RuntimePhase::Failed;
                state.desired_mode = mode;
                state.last_error = Some("内核没有返回运行会话".into());
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
                    });
                } else {
                    state.configuration = candidate;
                    state.revision = revision;
                    state.applied_mode = if explicit_stop { None } else { Some(mode) };
                    state.phase = if mode == RuntimeMode::Direct {
                        RuntimePhase::Stopped
                    } else {
                        RuntimePhase::Running
                    };
                    state.started_at = started_at;
                    state.session = session;
                    state.last_error = None;
                    self.backend.confirm_transition(before.session.as_ref());
                    self.publish(state.snapshot());
                }
                Ok(state.snapshot())
            }
            Err(error) => {
                *state = before;
                state.desired_mode = mode;
                state.phase = RuntimePhase::Failed;
                state.last_error = Some(error.message.clone());
                self.publish(state.snapshot());
                Err(error)
            }
        }
    }

    fn serialize_operation(&self) -> Result<std::sync::MutexGuard<'_, ()>, AppError> {
        self.operation.lock().map_err(|_| runtime_error())
    }
}

impl RuntimeCoordinator for ManagedRuntime {
    fn snapshot(&self) -> RuntimeSnapshot {
        let snapshot = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .snapshot();
        if snapshot.phase != RuntimePhase::Running {
            return snapshot;
        }
        // Do not block snapshots during a transition; the staged phase is
        // visible while the single writer owns the operation lock.
        let Ok(_guard) = self.operation.try_lock() else {
            return snapshot;
        };
        let session = {
            let state = self
                .state
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            if state.phase != RuntimePhase::Running {
                return state.snapshot();
            }
            state.session.clone()
        };
        let Some(session) = session else {
            return snapshot;
        };
        let health = self.backend.reconcile_session(&session);
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if state.session.as_ref() != Some(&session) || state.phase != RuntimePhase::Running {
            return state.snapshot();
        }
        let healthy = matches!(health, Ok(true));
        match health {
            Ok(true) => {}
            Ok(false) => {
                state.session = None;
                state.applied_mode = None;
                state.phase = RuntimePhase::Failed;
                state.started_at = None;
                state.last_error = Some("受管内核意外退出，系统代理已恢复".into());
            }
            Err(error) => {
                state.session = None;
                state.applied_mode = None;
                state.phase = RuntimePhase::Failed;
                state.started_at = None;
                // On a failed restoration the prior process is no longer a
                // usable owned session; never report its proxy as applied.
                state.last_error = Some(format!("内核异常或网络恢复失败：{}", error.message));
            }
        }
        if !healthy {
            self.publish(state.snapshot());
        }
        state.snapshot()
    }

    fn active_connections(&self) -> crate::observability::ActiveConnectionsSnapshot {
        use crate::observability::{parse_connections, ActiveConnectionsSnapshot};
        if self.snapshot().phase != RuntimePhase::Running {
            return ActiveConnectionsSnapshot::degraded();
        }
        self.backend
            .active_connections_json()
            .and_then(|value| parse_connections(&value))
            .unwrap_or_else(|_| ActiveConnectionsSnapshot::degraded())
    }

    fn recover_network(&self) -> Result<RuntimeSnapshot, AppError> {
        self.stop()?;
        match self.backend.restore_network() {
            Ok(()) => Ok(self.snapshot()),
            Err(error) => {
                self.report_startup_recovery_issue(error.message.clone());
                Err(error)
            }
        }
    }

    fn request_mode(&self, mode: RuntimeMode) -> Result<RuntimeSnapshot, AppError> {
        let _guard = self.serialize_operation()?;
        let configuration = self
            .state
            .lock()
            .map_err(|_| runtime_error())?
            .configuration
            .clone();
        self.transition(configuration, mode, false, false)
    }

    fn stop(&self) -> Result<RuntimeSnapshot, AppError> {
        let _guard = self.serialize_operation()?;
        let configuration = self
            .state
            .lock()
            .map_err(|_| runtime_error())?
            .configuration
            .clone();
        self.transition(configuration, RuntimeMode::Direct, true, false)
    }

    fn apply_configuration(
        &self,
        previous: &PersistedConfiguration,
        candidate: &PersistedConfiguration,
    ) -> Result<RuntimeSnapshot, AppError> {
        let _guard = self.serialize_operation()?;
        let state = self.state.lock().map_err(|_| runtime_error())?.clone();
        if state.configuration != *previous {
            return Err(AppError::storage("配置修订与运行时不一致"));
        }
        if previous == candidate {
            return Ok(state.snapshot());
        }
        let mode = match state.applied_mode {
            Some(RuntimeMode::Rules | RuntimeMode::Global)
                if candidate.active_profile_id.is_none() =>
            {
                RuntimeMode::Direct
            }
            Some(mode) => mode,
            None => RuntimeMode::Direct,
        };
        self.transition(candidate.clone(), mode, state.applied_mode.is_none(), true)
    }

    fn confirm_configuration(&self) -> RuntimeSnapshot {
        let _guard = self
            .operation
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if let Some(pending) = state.pending.take() {
            self.backend.confirm_transition(state.session.as_ref());
            state.configuration = pending.configuration;
            state.revision = pending.revision;
            state.desired_mode = pending.mode;
            state.applied_mode = if pending.explicit_stop {
                None
            } else {
                Some(pending.mode)
            };
            state.phase = if pending.mode == RuntimeMode::Direct {
                RuntimePhase::Stopped
            } else {
                RuntimePhase::Running
            };
            state.started_at = pending.started_at;
            state.session = pending.session;
            state.last_error = None;
            self.publish(state.snapshot());
        }
        state.snapshot()
    }

    fn restore_configuration(
        &self,
        previous: &PersistedConfiguration,
        snapshot: &RuntimeSnapshot,
    ) -> Result<RuntimeSnapshot, AppError> {
        let _guard = self.serialize_operation()?;
        let before = self.state.lock().map_err(|_| runtime_error())?.clone();
        if let Some(pending) = before.pending {
            if before.configuration != *previous || before.revision != snapshot.revision {
                return Err(AppError::storage("待回滚的配置修订不一致"));
            }
            self.backend
                .revert_transition(pending.session.as_ref(), before.session.as_ref())?;
            let mut state = self.state.lock().map_err(|_| runtime_error())?;
            state.pending = None;
            state.desired_mode = snapshot.desired_mode;
            state.phase = snapshot.phase;
            state.last_error = Some("配置保存失败，已恢复先前运行时".into());
            self.publish(state.snapshot());
            return Ok(state.snapshot());
        }
        let mode = snapshot.applied_mode.unwrap_or(RuntimeMode::Direct);
        self.transition(
            previous.clone(),
            mode,
            snapshot.applied_mode.is_none(),
            false,
        )?;
        let mut state = self.state.lock().map_err(|_| runtime_error())?;
        state.revision = snapshot.revision;
        state.desired_mode = snapshot.desired_mode;
        state.applied_mode = snapshot.applied_mode;
        state.phase = snapshot.phase;
        state.started_at = snapshot
            .runtime_uptime_ms
            .and_then(|elapsed| Instant::now().checked_sub(Duration::from_millis(elapsed)));
        state.last_error = Some("配置保存失败，已恢复先前运行时".into());
        self.publish(state.snapshot());
        Ok(state.snapshot())
    }
}

fn runtime_error() -> AppError {
    AppError {
        code: "runtime_error".into(),
        message: "代理运行时操作失败".into(),
        fields: Vec::new(),
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
