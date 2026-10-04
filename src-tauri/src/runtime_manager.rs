use super::*;
use crate::runtime_session::SessionLease;
use std::{sync::Mutex, time::Duration};

pub struct ManagedRuntime {
    state: Mutex<RuntimeState>,
    operation: Mutex<()>,
    events: Mutex<Option<tokio::sync::mpsc::UnboundedSender<RuntimeSnapshot>>>,
    backend: Box<dyn RuntimeBackend>,
    _ownership: SessionLease,
}

impl ManagedRuntime {
    /// 获取独占会话所有权并使用注入的后端创建运行时。
    pub fn new(
        configuration: PersistedConfiguration,
        backend: Box<dyn RuntimeBackend>,
        session_key: &str,
        selected_mode: RuntimeMode,
    ) -> Result<Self, AppError> {
        Self::from_lease(
            configuration,
            backend,
            SessionLease::acquire(session_key)?,
            selected_mode,
        )
    }

    pub(crate) fn from_lease(
        configuration: PersistedConfiguration,
        backend: Box<dyn RuntimeBackend>,
        ownership: SessionLease,
        selected_mode: RuntimeMode,
    ) -> Result<Self, AppError> {
        configuration.validate()?;
        let credential_versions = crate::runtime_plan::candidate_versions(
            &configuration,
            &std::collections::HashMap::new(),
            &[],
        );
        Ok(Self {
            state: Mutex::new(RuntimeState {
                configuration,
                revision: 0,
                configuration_revision: 0,
                runtime_plan_revision: 0,
                credential_versions,
                selected_mode,
                desired_mode: RuntimeMode::Direct,
                node: RuntimeStateNode::default(),
                history: history::TransitionHistory::default(),
                last_operation: OperationResult::default(),
                last_error: None,
                pending: None,
                pending_settings: None,
            }),
            operation: Mutex::new(()),
            events: Mutex::new(None),
            backend,
            _ownership: ownership,
        })
    }

    /// 最近 128 次脱敏状态转换，按时间顺序返回供诊断使用。
    pub fn transition_history(&self) -> Vec<TransitionRecord> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .history
            .snapshot()
    }

    pub fn subscribe(&self) -> tokio::sync::mpsc::UnboundedReceiver<RuntimeSnapshot> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        *self
            .events
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = Some(sender);
        receiver
    }

    pub(crate) fn with_configuration_revision(self, revision: u64) -> Self {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .configuration_revision = revision;
        self
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
        state
            .apply_event(RuntimeEvent::RecoveryBlocked {
                error: message.clone(),
            })
            .expect("startup recovery has no active session");
        state.begin_operation();
        state.fail_operation(message);
        self.publish(state.snapshot());
    }

    fn serialize_operation(&self) -> Result<std::sync::MutexGuard<'_, ()>, AppError> {
        self.operation.lock().map_err(|_| runtime_error())
    }
}

impl RuntimeCoordinator for ManagedRuntime {
    fn transition_history(&self) -> Vec<TransitionRecord> {
        ManagedRuntime::transition_history(self)
    }

    fn report_configuration_recovery_issue(&self, error: &AppError) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        state.begin_operation();
        state.fail_operation(error.message.clone());
        // Continue watching an existing healthy session despite cleanup errors.
        if state.session().is_none() {
            state
                .apply_event(RuntimeEvent::RecoveryBlocked {
                    error: error.message.clone(),
                })
                .expect("configuration recovery has no active session");
        }
        self.publish(state.snapshot());
    }
    fn snapshot(&self) -> RuntimeSnapshot {
        self.reconcile_snapshot()
    }

    fn active_connections(&self) -> crate::observability::ActiveConnectionsSnapshot {
        use crate::observability::{parse_connections, ActiveConnectionsSnapshot};
        let before = self.snapshot();
        if before.session_health != SessionHealth::Healthy {
            return ActiveConnectionsSnapshot::degraded();
        }
        let result = self
            .backend
            .active_connections_json()
            .and_then(|value| parse_connections(&value));
        let after = self.snapshot();
        if after.revision != before.revision || after.session_health != SessionHealth::Healthy {
            return ActiveConnectionsSnapshot::degraded();
        }
        result.unwrap_or_else(|error| {
            let mut degraded = ActiveConnectionsSnapshot::degraded();
            degraded.diagnostic = Some(error.message);
            degraded
        })
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
        let configuration = {
            let mut state = self.state.lock().map_err(|_| runtime_error())?;
            state.selected_mode = mode;
            state.configuration.clone()
        };
        self.transition(configuration, mode, false, false)
    }

    fn stop(&self) -> Result<RuntimeSnapshot, AppError> {
        let _guard = self.serialize_operation()?;
        let configuration = {
            let mut state = self.state.lock().map_err(|_| runtime_error())?;
            state.selected_mode = RuntimeMode::Direct;
            state.configuration.clone()
        };
        self.transition(configuration, RuntimeMode::Direct, true, false)
    }

    fn apply_configuration(
        &self,
        previous: &PersistedConfiguration,
        candidate: &PersistedConfiguration,
    ) -> Result<RuntimeSnapshot, AppError> {
        self.stage_configuration(previous, candidate, &[])
    }

    fn apply_configuration_with_credentials(
        &self,
        previous: &PersistedConfiguration,
        candidate: &PersistedConfiguration,
        changed_credentials: &[String],
    ) -> Result<RuntimeSnapshot, AppError> {
        self.stage_configuration(previous, candidate, changed_credentials)
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
        if let Some(configuration) = state.pending_settings.take() {
            state.configuration = configuration.configuration;
            state.credential_versions = configuration.credential_versions;
            state.configuration_revision += 1;
            state.revision += 1;
            state
                .apply_event(RuntimeEvent::MetadataCommitted)
                .expect("metadata commit preserves a stable lifecycle");
            state.complete_operation();
            self.publish(state.snapshot());
        }
        if let Some(pending) = state.pending.take() {
            self.backend.confirm_transition(state.session());
            state.configuration = pending.configuration;
            state.credential_versions = pending.credential_versions;
            state.runtime_plan_revision = pending.runtime_plan_revision;
            state.configuration_revision += 1;
            state.revision = pending.revision;
            state.desired_mode = pending.mode;
            state
                .commit_session(pending.explicit_stop)
                .expect("prepared candidate can be committed");
            state.complete_operation();
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
        if before.pending_settings.is_some() {
            let mut state = self.state.lock().map_err(|_| runtime_error())?;
            state.pending_settings = None;
            state.fail_operation("配置保存失败，已恢复先前运行时".into());
            self.publish(state.snapshot());
            return Ok(state.snapshot());
        }
        if let Some(pending) = before.pending.as_ref() {
            if before.configuration != *previous || before.revision != snapshot.revision {
                return Err(AppError::storage("待回滚的配置修订不一致"));
            }
            self.backend
                .revert_transition(pending.session.as_ref(), before.session())?;
            let mut state = self.state.lock().map_err(|_| runtime_error())?;
            state.pending = None;
            state.desired_mode = snapshot.desired_mode;
            state.apply_event(RuntimeEvent::Rollback {
                previous: Box::new(pending.previous_node.clone()),
            })?;
            state.fail_operation("配置保存失败，已恢复先前运行时".into());
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
        state.configuration_revision = snapshot.configuration_revision;
        state.runtime_plan_revision = snapshot.runtime_plan_revision;
        state.desired_mode = snapshot.desired_mode;
        let restored_start = snapshot
            .runtime_uptime_ms
            .and_then(|elapsed| Instant::now().checked_sub(Duration::from_millis(elapsed)));
        let active = state.session().cloned().map(|session| ActiveSession {
            session,
            mode,
            started_at: restored_start.unwrap_or_else(Instant::now),
        });
        let restored = match snapshot.phase {
            RuntimePhase::Running => RuntimeStateNode::Running {
                active: active.ok_or_else(runtime_error)?,
            },
            RuntimePhase::Failed => RuntimeStateNode::Failed {
                error: snapshot.last_error.clone().unwrap_or_default(),
                last_session: active,
                applied_mode: snapshot.applied_mode,
                health: snapshot.session_health,
            },
            _ => RuntimeStateNode::Stopped {
                applied_mode: snapshot.applied_mode,
                last_error: None,
            },
        };
        state.apply_event(RuntimeEvent::SnapshotRestored {
            previous: Box::new(restored),
        })?;
        state.fail_operation("配置保存失败，已恢复先前运行时".into());
        self.publish(state.snapshot());
        Ok(state.snapshot())
    }
}

fn runtime_error() -> AppError {
    AppError {
        code: "runtime_error".into(),
        message: "代理运行时操作失败".into(),
        fields: Vec::new(),
        context: None,
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;

#[path = "runtime_configuration.rs"]
mod configuration;

#[path = "runtime_health.rs"]
mod health;
#[path = "runtime_transition.rs"]
mod transition;
