use super::*;

fn active(pid: u32, mode: RuntimeMode) -> ActiveSession {
    ActiveSession {
        session: BackendSession {
            run_id: format!("run-{pid}"),
            process_id: pid,
            configuration_revision: 1,
            system_proxy_enabled: true,
            tun_enabled: false,
        },
        mode,
        started_at: Instant::now(),
    }
}
fn running() -> RuntimeStateNode {
    RuntimeStateNode::Running {
        active: active(1, RuntimeMode::Global),
    }
}
fn start() -> RuntimeStateNode {
    RuntimeStateNode::default()
        .transition(RuntimeEvent::StartRequested {
            mode: RuntimeMode::Global,
        })
        .expect("start request")
}
fn switching() -> RuntimeStateNode {
    running()
        .transition(RuntimeEvent::ModeSwitchRequested {
            mode: crate::models::TEST_RULES_MODE,
        })
        .expect("switch request")
}

#[test]
fn startup_and_switch_publish_only_committed_sessions() -> Result<(), RuntimeError> {
    let pending = start();
    assert_eq!(pending.phase(), RuntimePhase::Starting);
    assert_eq!(pending.active(), None);
    assert_eq!(pending.applied_mode(), None);
    let prepared = pending.transition(RuntimeEvent::ProcessStarted {
        active: active(1, RuntimeMode::Global),
    })?;
    assert_eq!(prepared.active(), None);
    let committed = prepared.transition(RuntimeEvent::HealthCheckPassed)?;
    assert_eq!(committed.phase(), RuntimePhase::Running);
    let pending = committed.transition(RuntimeEvent::ModeSwitchRequested {
        mode: crate::models::TEST_RULES_MODE,
    })?;
    let previous = pending.active().cloned();
    let prepared = pending.transition(RuntimeEvent::ProcessStarted {
        active: active(2, crate::models::TEST_RULES_MODE),
    })?;
    assert_eq!(prepared.active().cloned(), previous);
    assert_eq!(prepared.applied_mode(), Some(RuntimeMode::Global));
    let committed = prepared.transition(RuntimeEvent::HealthCheckPassed)?;
    assert_eq!(
        committed.applied_mode(),
        Some(crate::models::TEST_RULES_MODE)
    );
    assert_eq!(
        committed
            .active()
            .expect("committed session")
            .session
            .process_id,
        2
    );
    Ok(())
}

#[test]
fn failure_retains_healthy_session_and_supports_retry_and_metadata_commit(
) -> Result<(), RuntimeError> {
    let pending = switching();
    let failed = pending.transition(RuntimeEvent::OperationFailed {
        error: "候选内核启动失败".into(),
    })?;
    assert_eq!(failed.phase(), RuntimePhase::Failed);
    assert_eq!(failed.health(), SessionHealth::Healthy);
    assert_eq!(failed.applied_mode(), Some(RuntimeMode::Global));
    assert_eq!(failed.last_error(), Some("候选内核启动失败"));
    assert_eq!(
        failed
            .transition(RuntimeEvent::ModeSwitchRequested {
                mode: crate::models::TEST_RULES_MODE
            })?
            .phase(),
        RuntimePhase::Switching
    );
    assert_eq!(
        failed.transition(RuntimeEvent::MetadataCommitted)?.phase(),
        RuntimePhase::Running
    );
    let failed = RuntimeStateNode::default().transition(RuntimeEvent::OperationFailed {
        error: "配置无效".into(),
    })?;
    assert_eq!(failed.health(), SessionHealth::Inactive);
    assert_eq!(
        failed
            .transition(RuntimeEvent::StartRequested {
                mode: RuntimeMode::Global
            })?
            .phase(),
        RuntimePhase::Starting
    );
    assert_eq!(
        failed.transition(RuntimeEvent::MetadataCommitted)?,
        RuntimeStateNode::default()
    );
    Ok(())
}

#[test]
fn stopping_and_direct_mode_keep_distinct_applied_modes_and_preserve_failed_stop(
) -> Result<(), RuntimeError> {
    let original = running();
    let stopping = original.transition(RuntimeEvent::StopRequested)?;
    assert_eq!(stopping.phase(), RuntimePhase::Recovering);
    assert_eq!(stopping.active(), original.active());
    let failed = stopping.transition(RuntimeEvent::RecoveryFailed {
        error: "恢复失败".into(),
    })?;
    assert_eq!(failed.active(), original.active());
    assert_eq!(failed.health(), SessionHealth::Healthy);
    let stopped = stopping.transition(RuntimeEvent::RecoverySucceeded {
        explicit_stop: true,
    })?;
    assert_eq!(stopped, RuntimeStateNode::default());
    let direct = original
        .transition(RuntimeEvent::ModeSwitchRequested {
            mode: RuntimeMode::Direct,
        })?
        .transition(RuntimeEvent::RecoverySucceeded {
            explicit_stop: false,
        })?;
    assert_eq!(direct.phase(), RuntimePhase::Stopped);
    assert_eq!(direct.applied_mode(), Some(RuntimeMode::Direct));
    let starting = direct.transition(RuntimeEvent::StartRequested {
        mode: RuntimeMode::Global,
    })?;
    assert_eq!(starting.applied_mode(), Some(RuntimeMode::Direct));
    let failed = starting.transition(RuntimeEvent::OperationFailed {
        error: "启动失败".into(),
    })?;
    assert_eq!(failed.applied_mode(), Some(RuntimeMode::Direct));
    Ok(())
}

#[test]
fn exited_session_never_reports_proxy_as_applied_and_requires_failed_recovery(
) -> Result<(), RuntimeError> {
    for error in [None, Some("所有权发生变化".into())] {
        let failed = running().transition(RuntimeEvent::ProcessExited {
            recovery_error: error.clone(),
        })?;
        assert_eq!(failed.active(), None);
        assert_eq!(failed.applied_mode(), None);
        assert_eq!(
            failed.health(),
            if error.is_some() {
                SessionHealth::RecoveryRequired
            } else {
                SessionHealth::Exited
            }
        );
        assert_eq!(failed.transition(RuntimeEvent::MetadataCommitted)?, failed);
    }
    let blocked = RuntimeStateNode::default().transition(RuntimeEvent::RecoveryBlocked {
        error: "恢复阻塞".into(),
    })?;
    assert_eq!(blocked.health(), SessionHealth::RecoveryRequired);
    assert_eq!(
        blocked
            .transition(RuntimeEvent::StopRequested)?
            .transition(RuntimeEvent::RecoverySucceeded {
                explicit_stop: true
            })?,
        RuntimeStateNode::default()
    );
    Ok(())
}

#[test]
fn rollback_restores_the_exact_prior_lifecycle_after_backend_reverts() -> Result<(), RuntimeError> {
    let previous = running();
    let prepared = previous
        .transition(RuntimeEvent::ModeSwitchRequested {
            mode: crate::models::TEST_RULES_MODE,
        })?
        .transition(RuntimeEvent::ProcessStarted {
            active: active(2, crate::models::TEST_RULES_MODE),
        })?;
    assert_eq!(
        prepared.transition(RuntimeEvent::Rollback {
            previous: Box::new(previous.clone())
        })?,
        previous
    );
    let failed = prepared.transition(RuntimeEvent::OperationFailed {
        error: "持久化失败".into(),
    })?;
    assert_eq!(
        failed.transition(RuntimeEvent::Rollback {
            previous: Box::new(previous.clone())
        })?,
        previous
    );
    assert!(previous
        .transition(RuntimeEvent::Rollback {
            previous: Box::new(start())
        })
        .is_err());
    Ok(())
}

#[test]
fn invalid_events_are_rejected_without_losing_the_original_node() {
    for node in [
        RuntimeStateNode::default(),
        start(),
        running(),
        switching(),
        running()
            .transition(RuntimeEvent::StopRequested)
            .expect("stop"),
    ] {
        for event in [
            RuntimeEvent::HealthCheckPassed,
            RuntimeEvent::RecoverySucceeded {
                explicit_stop: true,
            },
        ] {
            let before = node.clone();
            if !matches!(
                (&node, &event),
                (
                    RuntimeStateNode::Recovering { .. },
                    RuntimeEvent::RecoverySucceeded { .. }
                )
            ) {
                assert!(matches!(
                    node.transition(event),
                    Err(RuntimeError::InvalidTransition { .. })
                ));
                assert_eq!(node, before);
            }
        }
    }
    assert!(running()
        .transition(RuntimeEvent::StartRequested {
            mode: RuntimeMode::Global
        })
        .is_err());
    assert!(RuntimeStateNode::default()
        .transition(RuntimeEvent::ModeSwitchRequested {
            mode: RuntimeMode::Global
        })
        .is_err());
    assert!(start().transition(RuntimeEvent::StopRequested).is_err());
    assert!(switching()
        .transition(RuntimeEvent::ProcessExited {
            recovery_error: None
        })
        .is_err());
    assert!(running()
        .transition(RuntimeEvent::RecoveryBlocked {
            error: "blocked".into()
        })
        .is_err());
    assert!(running()
        .transition(RuntimeEvent::ProcessStarted {
            active: active(2, crate::models::TEST_RULES_MODE)
        })
        .is_err());
}

#[test]
fn invariants_reject_invalid_identity_mode_and_health_before_or_after_transition() {
    for invalid in [
        RuntimeStateNode::Running {
            active: active(0, RuntimeMode::Global),
        },
        RuntimeStateNode::Running {
            active: active(1, RuntimeMode::Direct),
        },
        RuntimeStateNode::Stopped {
            applied_mode: Some(RuntimeMode::Global),
            last_error: None,
        },
        RuntimeStateNode::Failed {
            error: "failed".into(),
            last_session: Some(active(1, RuntimeMode::Global)),
            applied_mode: Some(crate::models::TEST_RULES_MODE),
            health: SessionHealth::Healthy,
        },
        RuntimeStateNode::Failed {
            error: "failed".into(),
            last_session: None,
            applied_mode: None,
            health: SessionHealth::Healthy,
        },
        RuntimeStateNode::Starting {
            mode: RuntimeMode::Direct,
            candidate: None,
            previous_applied_mode: None,
            previous_health: SessionHealth::Inactive,
        },
    ] {
        assert!(matches!(
            invalid.verify_invariants(),
            Err(RuntimeError::InvariantViolated(_))
        ));
        assert!(matches!(
            invalid.transition(RuntimeEvent::MetadataCommitted),
            Err(RuntimeError::InvariantViolated(_))
        ));
    }
    let mut empty_identity = active(1, RuntimeMode::Global);
    empty_identity.session.run_id.clear();
    assert!(RuntimeStateNode::Running {
        active: empty_identity
    }
    .verify_invariants()
    .is_err());
    let node = start();
    let unchanged = node.clone();
    assert!(matches!(
        node.transition(RuntimeEvent::ProcessStarted {
            active: active(2, crate::models::TEST_RULES_MODE)
        }),
        Err(RuntimeError::InvariantViolated(_))
    ));
    assert_eq!(node, unchanged);
    let prepared = node
        .transition(RuntimeEvent::ProcessStarted {
            active: active(2, RuntimeMode::Global),
        })
        .expect("prepared");
    assert!(prepared
        .transition(RuntimeEvent::ProcessStarted {
            active: active(3, RuntimeMode::Global)
        })
        .is_err());
}

#[test]
fn confirmed_snapshot_restoration_requires_stable_valid_nodes() -> Result<(), RuntimeError> {
    let previous = running();
    let stopped = RuntimeStateNode::default();
    assert_eq!(
        stopped.transition(RuntimeEvent::SnapshotRestored {
            previous: Box::new(previous.clone())
        })?,
        previous
    );
    assert!(start()
        .transition(RuntimeEvent::SnapshotRestored {
            previous: Box::new(previous)
        })
        .is_err());
    assert!(stopped
        .transition(RuntimeEvent::SnapshotRestored {
            previous: Box::new(start())
        })
        .is_err());
    assert!(stopped
        .transition(RuntimeEvent::SnapshotRestored {
            previous: Box::new(RuntimeStateNode::Stopped {
                applied_mode: Some(RuntimeMode::Global),
                last_error: None
            })
        })
        .is_err());
    Ok(())
}
