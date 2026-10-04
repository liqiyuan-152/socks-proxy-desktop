//! 有界、脱敏的运行时转换历史，不记录配置、会话身份或错误文本。
use crate::{runtime::RuntimePhase, runtime_state_machine::RuntimeEvent};
use serde::Serialize;
use std::{collections::VecDeque, time::SystemTime};

const HISTORY_LIMIT: usize = 128;

/// 单次已接受或拒绝的生命周期事件。
#[derive(Clone, Debug, Serialize)]
pub struct TransitionRecord {
    pub timestamp_ms: u64,
    pub operation_id: u64,
    pub from: RuntimePhase,
    pub to: RuntimePhase,
    pub event: &'static str,
    pub accepted: bool,
}

#[derive(Clone, Default)]
pub(super) struct TransitionHistory(VecDeque<TransitionRecord>);

impl TransitionHistory {
    pub(super) fn record(
        &mut self,
        operation_id: u64,
        from: RuntimePhase,
        to: RuntimePhase,
        event: &RuntimeEvent,
        accepted: bool,
    ) {
        let event = match event {
            RuntimeEvent::StartRequested { .. } => "start_requested",
            RuntimeEvent::ModeSwitchRequested { .. } => "mode_switch_requested",
            RuntimeEvent::ProcessStarted { .. } => "process_started",
            RuntimeEvent::HealthCheckPassed => "health_check_passed",
            RuntimeEvent::ProcessExited { .. } => "process_exited",
            RuntimeEvent::RecoverySucceeded { .. } => "recovery_succeeded",
            RuntimeEvent::RecoveryFailed { .. } => "recovery_failed",
            RuntimeEvent::OperationFailed { .. } => "operation_failed",
            RuntimeEvent::RecoveryBlocked { .. } => "recovery_blocked",
            RuntimeEvent::StopRequested => "stop_requested",
            RuntimeEvent::Rollback { .. } => "rollback",
            RuntimeEvent::SnapshotRestored { .. } => "snapshot_restored",
            RuntimeEvent::MetadataCommitted => "metadata_committed",
        };
        if event == "metadata_committed" && accepted {
            tracing::debug!(target: "runtime_transition", operation_id, ?from, ?to, event, accepted, "runtime metadata committed");
        } else {
            let span = tracing::info_span!("runtime_transition", operation_id);
            let _entered = span.enter();
            tracing::info!(?from, ?to, event, accepted, "runtime state changed");
        }
        if self.0.len() == HISTORY_LIMIT {
            self.0.pop_front();
        }
        self.0.push_back(TransitionRecord {
            timestamp_ms: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_millis() as u64),
            operation_id,
            from,
            to,
            event,
            accepted,
        });
    }

    pub(super) fn snapshot(&self) -> Vec<TransitionRecord> {
        self.0.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_is_bounded_and_contains_no_event_payload() -> Result<(), serde_json::Error> {
        let mut history = TransitionHistory::default();
        let event = RuntimeEvent::OperationFailed {
            error: "private-password-and-path".into(),
        };
        for id in 0..140 {
            history.record(
                id,
                RuntimePhase::Running,
                RuntimePhase::Failed,
                &event,
                true,
            );
        }
        let records = history.snapshot();
        assert_eq!(records.len(), HISTORY_LIMIT);
        assert_eq!(records[0].operation_id, 12);
        assert_eq!(records[127].operation_id, 139);
        assert!(!serde_json::to_string(&records)?.contains("private-password-and-path"));
        Ok(())
    }
}
