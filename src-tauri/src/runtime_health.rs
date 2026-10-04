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
            state.session().cloned()
        };
        let Some(session) = session else {
            return snapshot;
        };
        let health = self.backend.reconcile_session(&session);
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if state.session() != Some(&session) {
            return state.snapshot();
        }
        let healthy = matches!(health, Ok(true));
        match health {
            Ok(true) => {}
            Ok(false) => {
                state
                    .apply_event(RuntimeEvent::ProcessExited {
                        recovery_error: None,
                    })
                    .expect("reconciled committed session can exit");
            }
            Err(error) => {
                state
                    .apply_event(RuntimeEvent::ProcessExited {
                        recovery_error: Some(error.message),
                    })
                    .expect("reconciled committed session can fail recovery");
            }
        }

        if !healthy {
            state.last_error = None;
            self.publish(state.snapshot());
        }
        state.snapshot()
    }
}
