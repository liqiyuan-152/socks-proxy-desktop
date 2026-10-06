use super::*;

fn active() -> ActiveSession {
    ActiveSession {
        session: BackendSession {
            run_id: "owned".into(),
            process_id: 42,
            configuration_revision: 1,
            system_proxy_enabled: true,
            tun_enabled: false,
        },
        mode: RuntimeMode::Global,
        started_at: Instant::now(),
    }
}

#[test]
fn every_event_is_checked_for_each_lifecycle_shape() -> Result<(), RuntimeError> {
    let a = active();
    let stable_running = RuntimeStateNode::Running { active: a.clone() };
    let nodes = [
        RuntimeStateNode::default(),
        RuntimeStateNode::Stopped {
            applied_mode: Some(RuntimeMode::Direct),
            last_error: Some("old".into()),
        },
        RuntimeStateNode::Starting {
            mode: RuntimeMode::Global,
            candidate: None,
            previous_health: SessionHealth::Inactive,
            previous_applied_mode: None,
        },
        RuntimeStateNode::Starting {
            mode: RuntimeMode::Global,
            candidate: Some(a.clone()),
            previous_health: SessionHealth::Exited,
            previous_applied_mode: None,
        },
        stable_running.clone(),
        RuntimeStateNode::Switching {
            current: a.clone(),
            mode: RuntimeMode::Global,
            candidate: None,
        },
        RuntimeStateNode::Switching {
            current: a.clone(),
            mode: RuntimeMode::Global,
            candidate: Some(a.clone()),
        },
        RuntimeStateNode::Recovering {
            failed_session: Some(a.clone()),
            applied_mode: Some(RuntimeMode::Global),
            health: SessionHealth::Healthy,
            attempt: 0,
        },
        RuntimeStateNode::Recovering {
            failed_session: None,
            applied_mode: None,
            health: SessionHealth::RecoveryRequired,
            attempt: 1,
        },
        RuntimeStateNode::Failed {
            error: "switch".into(),
            last_session: Some(a.clone()),
            applied_mode: Some(RuntimeMode::Global),
            health: SessionHealth::Healthy,
        },
        RuntimeStateNode::Failed {
            error: "exit".into(),
            last_session: None,
            applied_mode: None,
            health: SessionHealth::Exited,
        },
        RuntimeStateNode::Failed {
            error: "restore".into(),
            last_session: None,
            applied_mode: None,
            health: SessionHealth::RecoveryRequired,
        },
    ];
    for node in nodes {
        let stable = matches!(
            node.phase(),
            RuntimePhase::Stopped | RuntimePhase::Running | RuntimePhase::Failed
        );
        let has_active = node.active().is_some();
        let empty_candidate = matches!(
            node,
            RuntimeStateNode::Starting {
                candidate: None,
                ..
            } | RuntimeStateNode::Switching {
                candidate: None,
                ..
            }
        );
        let prepared = matches!(
            node,
            RuntimeStateNode::Starting {
                candidate: Some(_),
                ..
            } | RuntimeStateNode::Switching {
                candidate: Some(_),
                ..
            }
        );
        let recovering = node.phase() == RuntimePhase::Recovering;
        let cases = [
            (
                RuntimeEvent::StartRequested {
                    mode: RuntimeMode::Global,
                },
                stable && !has_active,
            ),
            (
                RuntimeEvent::StartRequested {
                    mode: RuntimeMode::Direct,
                },
                stable && !has_active,
            ),
            (
                RuntimeEvent::ModeSwitchRequested {
                    mode: crate::models::TEST_RULES_MODE,
                },
                stable && has_active,
            ),
            (
                RuntimeEvent::ModeSwitchRequested {
                    mode: RuntimeMode::Direct,
                },
                stable && has_active,
            ),
            (RuntimeEvent::StopRequested, stable),
            (
                RuntimeEvent::ProcessStarted { active: a.clone() },
                empty_candidate,
            ),
            (RuntimeEvent::HealthCheckPassed, prepared),
            (
                RuntimeEvent::ProcessExited {
                    recovery_error: None,
                },
                stable && has_active,
            ),
            (
                RuntimeEvent::ProcessExited {
                    recovery_error: Some("restore".into()),
                },
                stable && has_active,
            ),
            (
                RuntimeEvent::RecoverySucceeded {
                    explicit_stop: true,
                },
                recovering,
            ),
            (
                RuntimeEvent::RecoverySucceeded {
                    explicit_stop: false,
                },
                recovering,
            ),
            (
                RuntimeEvent::RecoveryFailed {
                    error: "restore".into(),
                },
                recovering,
            ),
            (
                RuntimeEvent::OperationFailed {
                    error: "operation".into(),
                },
                true,
            ),
            (
                RuntimeEvent::RecoveryBlocked {
                    error: "blocked".into(),
                },
                !has_active,
            ),
            (
                RuntimeEvent::Rollback {
                    previous: Box::new(stable_running.clone()),
                },
                !stable || node.phase() == RuntimePhase::Failed,
            ),
            (
                RuntimeEvent::SnapshotRestored {
                    previous: Box::new(stable_running.clone()),
                },
                stable,
            ),
            (RuntimeEvent::MetadataCommitted, stable),
        ];
        for (event, accepted) in cases {
            let before = node.clone();
            let result = node.transition(event.clone());
            assert_eq!(
                result.is_ok(),
                accepted,
                "node={node:?}, event={event:?}, result={result:?}"
            );
            assert_eq!(node, before);
            if let Ok(next) = result {
                next.verify_invariants()?;
            } else {
                assert!(matches!(
                    result,
                    Err(RuntimeError::InvalidTransition { .. })
                ));
            }
        }
    }
    Ok(())
}
