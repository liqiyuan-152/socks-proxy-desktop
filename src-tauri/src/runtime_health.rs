use super::*;

impl ManagedRuntime {
    pub(super) fn reconcile_snapshot(&self) -> RuntimeSnapshot {
        let snapshot = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .snapshot();
        if snapshot.session_health != SessionHealth::Healthy {
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
            if state.pending.is_some() || state.pending_settings.is_some() {
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
        if state.session.as_ref() != Some(&session) {
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
                state.session_health = SessionHealth::Exited;
                state.last_error = Some("受管内核意外退出，系统代理已恢复".into());
            }
            Err(error) => {
                state.session = None;
                state.applied_mode = None;
                state.phase = RuntimePhase::Failed;
                state.started_at = None;
                // On a failed restoration the prior process is no longer a
                // usable owned session; never report its proxy as applied.
                state.session_health = SessionHealth::RecoveryRequired;
                state.last_error = Some(format!("内核异常或网络恢复失败：{}", error.message));
            }
        }
        if !healthy {
            self.publish(state.snapshot());
        }
        state.snapshot()
    }
}
