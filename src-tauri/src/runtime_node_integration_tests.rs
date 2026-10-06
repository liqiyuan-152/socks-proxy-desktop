use super::*;

#[test]
fn rejected_candidate_is_reverted_and_old_session_remains_healthy() -> Result<(), AppError> {
    struct InvalidCandidate {
        inner: Arc<FakeBackend>,
        reverted: Arc<AtomicU32>,
    }
    impl RuntimeBackend for InvalidCandidate {
        fn transition(
            &self,
            previous: Option<&BackendSession>,
            candidate: &PersistedConfiguration,
            mode: RuntimeMode,
            revision: u64,
        ) -> Result<Option<BackendSession>, AppError> {
            let mut session = self.inner.transition(previous, candidate, mode, revision)?;
            if matches!(mode, RuntimeMode::Rules { .. }) {
                if let Some(session) = session.as_mut() {
                    session.process_id = 0;
                }
            }
            Ok(session)
        }
        fn confirm_transition(&self, previous: Option<&BackendSession>) {
            self.inner.confirm_transition(previous);
        }
        fn revert_transition(
            &self,
            candidate: Option<&BackendSession>,
            previous: Option<&BackendSession>,
        ) -> Result<(), AppError> {
            assert_eq!(candidate.map(|session| session.process_id), Some(0));
            assert!(previous.is_some());
            self.reverted.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn reconcile_session(&self, session: &BackendSession) -> Result<bool, AppError> {
            self.inner.reconcile_session(session)
        }
    }
    let reverted = Arc::new(AtomicU32::new(0));
    let runtime = ManagedRuntime::from_lease(
        configuration(),
        Box::new(InvalidCandidate {
            inner: Arc::new(FakeBackend::default()),
            reverted: reverted.clone(),
        }),
        SessionLease::acquire(&uuid::Uuid::new_v4().to_string())?,
        RuntimeMode::Direct,
    )?;
    let before = runtime.request_mode(RuntimeMode::Global)?;
    assert_eq!(
        runtime
            .request_mode(crate::models::TEST_RULES_MODE)
            .expect_err("invalid process identity")
            .code,
        "runtime_invariant_violated"
    );
    let after = runtime.snapshot();
    assert_eq!(after.phase, RuntimePhase::Failed);
    assert_eq!(after.session_health, SessionHealth::Healthy);
    assert_eq!(after.applied_mode, before.applied_mode);
    assert_eq!(after.revision, before.revision);
    assert_eq!(reverted.load(Ordering::SeqCst), 1);
    let history = runtime.transition_history();
    assert!(history
        .iter()
        .any(|record| record.event == "process_started" && !record.accepted));
    assert_eq!(
        history.last().map(|record| record.event),
        Some("operation_failed")
    );
    Ok(())
}

#[test]
fn durable_rollback_restores_node_and_keeps_history() -> Result<(), AppError> {
    let runtime = manager(Arc::new(FakeBackend::default()), configuration());
    let before = runtime.request_mode(RuntimeMode::Global)?;
    let configuration = configuration();
    let mut candidate = configuration.clone();
    candidate.profiles[0].port += 1;
    runtime.apply_configuration(&configuration, &candidate)?;
    let staged = runtime.snapshot();
    assert_eq!(staged.phase, RuntimePhase::Switching);
    assert_eq!(staged.revision, before.revision);
    let restored = runtime.restore_configuration(&configuration, &before)?;
    assert_eq!(restored.phase, before.phase);
    assert_eq!(restored.applied_mode, before.applied_mode);
    assert_eq!(restored.runtime_plan_revision, before.runtime_plan_revision);
    assert_eq!(restored.session_health, before.session_health);
    let history = runtime.transition_history();
    assert_eq!(history.last().map(|record| record.event), Some("rollback"));
    assert!(history
        .iter()
        .any(|record| record.from == RuntimePhase::Switching && record.event == "process_started"));
    Ok(())
}

#[test]
fn restoration_without_pending_candidate_reapplies_and_restores_snapshot() -> Result<(), AppError> {
    for phase in [
        RuntimePhase::Running,
        RuntimePhase::Failed,
        RuntimePhase::Stopped,
    ] {
        let backend = Arc::new(FakeBackend::default());
        let runtime = manager(backend.clone(), configuration());
        let configuration = configuration();
        runtime.request_mode(RuntimeMode::Global)?;
        if phase == RuntimePhase::Failed {
            backend.fail_next.store(true, Ordering::SeqCst);
            assert!(runtime
                .request_mode(crate::models::TEST_RULES_MODE)
                .is_err());
        } else if phase == RuntimePhase::Stopped {
            runtime.stop()?;
        }
        let before = runtime.snapshot();
        runtime.request_mode(crate::models::TEST_RULES_MODE)?;
        let restored = runtime.restore_configuration(&configuration, &before)?;
        assert_eq!(restored.phase, before.phase);
        assert_eq!(restored.applied_mode, before.applied_mode);
        assert_eq!(restored.session_health, before.session_health);
        assert_eq!(restored.revision, before.revision);
        assert_eq!(restored.runtime_plan_revision, before.runtime_plan_revision);
        assert_eq!(
            restored.configuration_revision,
            before.configuration_revision
        );
        assert_eq!(restored.desired_mode, before.desired_mode);
        assert_eq!(
            restored.runtime_uptime_ms.is_some(),
            before.runtime_uptime_ms.is_some()
        );
        assert_eq!(
            runtime
                .transition_history()
                .last()
                .map(|record| record.event),
            Some("snapshot_restored")
        );
    }
    Ok(())
}
