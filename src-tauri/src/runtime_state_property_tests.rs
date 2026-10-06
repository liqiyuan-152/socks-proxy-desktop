use super::*;
use proptest::prelude::*;

fn session(mode: RuntimeMode, process_id: u32) -> ActiveSession {
    ActiveSession {
        session: BackendSession {
            run_id: format!("generated-{process_id}"),
            process_id,
            configuration_revision: 1,
            system_proxy_enabled: true,
            tun_enabled: false,
        },
        mode,
        started_at: Instant::now(),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn arbitrary_event_sequences_preserve_invariants(events in prop::collection::vec(0u8..14, 0..100)) {
        let mut node = RuntimeStateNode::default();
        for choice in events {
            let mode = match &node {
                RuntimeStateNode::Starting { mode, .. } | RuntimeStateNode::Switching { mode, .. } => *mode,
                _ => RuntimeMode::Global,
            };
            let event = match choice {
                0 => RuntimeEvent::StartRequested { mode: RuntimeMode::Global },
                1 => RuntimeEvent::StartRequested { mode: RuntimeMode::Direct },
                2 => RuntimeEvent::ModeSwitchRequested { mode: crate::models::TEST_RULES_MODE },
                3 => RuntimeEvent::StopRequested,
                4 => RuntimeEvent::ProcessStarted { active: session(mode, 42) },
                5 => RuntimeEvent::HealthCheckPassed,
                6 => RuntimeEvent::ProcessExited { recovery_error: None },
                7 => RuntimeEvent::RecoverySucceeded { explicit_stop: true },
                8 => RuntimeEvent::RecoverySucceeded { explicit_stop: false },
                9 => RuntimeEvent::RecoveryFailed { error: "generated".into() },
                10 => RuntimeEvent::OperationFailed { error: "generated".into() },
                11 => RuntimeEvent::RecoveryBlocked { error: "generated".into() },
                12 => RuntimeEvent::Rollback { previous: Box::new(RuntimeStateNode::default()) },
                _ => RuntimeEvent::MetadataCommitted,
            };
            let previous = node.clone();
            let result = node.transition(event.clone());
            prop_assert_eq!(&node, &previous);
            if let Ok(next) = result {
                prop_assert!(next.verify_invariants().is_ok());
                if matches!(event, RuntimeEvent::ProcessStarted { .. }) {
                    prop_assert_eq!(next.applied_mode(), previous.applied_mode());
                    prop_assert_eq!(next.active(), previous.active());
                }
                if matches!(event, RuntimeEvent::ProcessExited { .. }) {
                    prop_assert!(next.active().is_none());
                    prop_assert!(next.applied_mode().is_none());
                }
                node = next;
            }
            prop_assert!(node.verify_invariants().is_ok());
        }
    }

    #[test]
    fn rejected_candidate_keeps_committed_session(process_id in any::<u32>()) {
        let running = RuntimeStateNode::Running { active: session(RuntimeMode::Global, 42) };
        let switching = running.transition(RuntimeEvent::ModeSwitchRequested { mode: crate::models::TEST_RULES_MODE }).expect("valid switch request");
        let result = switching.transition(RuntimeEvent::ProcessStarted { active: session(RuntimeMode::Global, process_id) });
        prop_assert!(matches!(result, Err(RuntimeError::InvariantViolated(_))));
        prop_assert_eq!(switching.active(), running.active());
        prop_assert_eq!(switching.applied_mode(), Some(RuntimeMode::Global));
    }
}
