use crate::{
    runtime::{RuntimePhase, RuntimeSnapshot},
    store::{RuntimeDiagnostic, SqliteConfigurationStore},
};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};

pub const SNAPSHOT_EVENT: &str = "runtime://snapshot";

#[derive(Default)]
pub struct SnapshotTracker {
    last: Option<RuntimeSnapshot>,
}

impl SnapshotTracker {
    pub fn changed(&mut self, snapshot: &RuntimeSnapshot) -> bool {
        let mut comparable = snapshot.clone();
        comparable.runtime_uptime_ms = None;
        if self.last.as_ref() == Some(&comparable) {
            return false;
        }
        self.last = Some(comparable);
        true
    }
}

fn event_description(snapshot: &RuntimeSnapshot) -> (&'static str, &'static str) {
    match snapshot.phase {
        RuntimePhase::Starting => ("info", "代理内核正在启动"),
        RuntimePhase::Running => ("info", "代理模式已提交"),
        RuntimePhase::Switching => ("info", "代理模式正在切换"),
        RuntimePhase::Recovering => ("info", "正在恢复系统代理"),
        RuntimePhase::Stopped => ("info", "代理运行时已停止"),
        RuntimePhase::Failed => ("error", "运行时切换或内核状态异常，请查看当前快照"),
    }
}

/// Polls the authoritative coordinator (which also reconciles unexpected
/// child exit), emits state transitions, and stores only static diagnostics.
pub fn start<R: Runtime>(
    app: AppHandle<R>,
    store: Arc<SqliteConfigurationStore>,
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<RuntimeSnapshot>,
) {
    use crate::configuration_service::ConfigurationService;
    use tauri::Manager;
    tauri::async_runtime::spawn(async move {
        let mut timer = tokio::time::interval(std::time::Duration::from_millis(250));
        let mut tracker = SnapshotTracker::default();
        loop {
            let snapshot = tokio::select! {
                biased;
                Some(snapshot) = receiver.recv() => snapshot,
                _ = timer.tick() => {
                    let service = app.state::<Arc<ConfigurationService>>().inner().clone();
                    match tauri::async_runtime::spawn_blocking(move || service.runtime_snapshot()).await {
                        Ok(snapshot) => snapshot,
                        Err(_) => break,
                    }
                }
            };
            if !tracker.changed(&snapshot) {
                continue;
            }
            crate::tray::sync_snapshot(&app, &snapshot);
            let (severity, summary) = event_description(&snapshot);
            let created_at_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis() as i64);
            if let Ok(created_at_ms) = created_at_ms {
                let store = store.clone();
                let diagnostic = RuntimeDiagnostic {
                    id: uuid::Uuid::new_v4().to_string(),
                    created_at_ms,
                    severity: severity.into(),
                    summary: summary.into(),
                };
                let _ = tauri::async_runtime::spawn_blocking(move || {
                    store.record_diagnostic(&diagnostic)
                })
                .await;
            }
            let _ = app.emit(SNAPSHOT_EVENT, &snapshot);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{models::RuntimeMode, runtime::TrafficCoverage};

    #[test]
    fn emits_only_state_transitions_not_uptime_ticks_or_sensitive_diagnostics() {
        let mut tracker = SnapshotTracker::default();
        let mut snapshot = RuntimeSnapshot {
            revision: 1,
            selected_mode: RuntimeMode::Global,
            desired_mode: RuntimeMode::Global,
            applied_mode: Some(RuntimeMode::Global),
            phase: RuntimePhase::Running,
            active_profile_id: Some("profile-id".into()),
            runtime_uptime_ms: Some(0),
            system_proxy_enabled: true,
            tun_enabled: false,
            coverage: TrafficCoverage::SystemProxyApps,
            last_error: None,
        };
        assert!(tracker.changed(&snapshot));
        snapshot.runtime_uptime_ms = Some(1_000);
        assert!(!tracker.changed(&snapshot));
        snapshot.phase = RuntimePhase::Failed;
        snapshot.applied_mode = None;
        assert!(tracker.changed(&snapshot));
        assert!(!event_description(&snapshot).1.contains("password"));
    }
}
