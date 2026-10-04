use crate::{
    error::AppError,
    models::{PersistedConfiguration, RuntimeMode},
    observability::ActiveConnectionsSnapshot,
};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum SessionHealth {
    Inactive,
    Healthy,
    Exited,
    RecoveryRequired,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum OperationOutcome {
    #[default]
    Idle,
    Pending,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct OperationResult {
    pub id: u64,
    pub outcome: OperationOutcome,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum RuntimePhase {
    Stopped,
    Starting,
    Running,
    Switching,
    Recovering,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum TrafficCoverage {
    None,
    SystemProxyApps,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct RuntimeSnapshot {
    pub revision: u64,
    pub configuration_revision: u64,
    pub runtime_plan_revision: u64,
    pub selected_mode: RuntimeMode,
    pub desired_mode: RuntimeMode,
    pub applied_mode: Option<RuntimeMode>,
    pub phase: RuntimePhase,
    pub session_health: SessionHealth,
    pub last_operation: OperationResult,
    pub active_profile_id: Option<String>,
    pub runtime_uptime_ms: Option<u64>,
    pub system_proxy_enabled: bool,
    pub tun_enabled: bool,
    pub coverage: TrafficCoverage,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendSession {
    pub run_id: String,
    pub process_id: u32,
    pub configuration_revision: u64,
    pub system_proxy_enabled: bool,
    pub tun_enabled: bool,
}

pub trait RuntimeBackend: Send + Sync {
    // On failure, the previous session and system proxy must remain applied.
    // On success, retain the previous session until confirm or revert.
    fn transition(
        &self,
        previous: Option<&BackendSession>,
        candidate: &PersistedConfiguration,
        mode: RuntimeMode,
        revision: u64,
    ) -> Result<Option<BackendSession>, AppError>;
    fn confirm_transition(&self, previous: Option<&BackendSession>);
    fn revert_transition(
        &self,
        candidate: Option<&BackendSession>,
        previous: Option<&BackendSession>,
    ) -> Result<(), AppError>;
    /// Reconcile a committed session; false means the owned process has exited.
    fn reconcile_session(&self, session: &BackendSession) -> Result<bool, AppError>;
    fn active_connections_json(&self) -> Result<serde_json::Value, AppError> {
        Err(AppError::unavailable("当前内核不提供活跃连接接口"))
    }
    fn restore_network(&self) -> Result<(), AppError> {
        Err(AppError::unavailable("当前平台不支持系统代理恢复"))
    }
}

#[derive(Clone)]
struct PendingConfiguration {
    configuration: PersistedConfiguration,
    revision: u64,
    mode: RuntimeMode,
    explicit_stop: bool,
    session: Option<BackendSession>,
    started_at: Option<Instant>,
    credential_versions: HashMap<String, String>,
    runtime_plan_revision: u64,
}

#[derive(Clone)]
struct PendingMetadata {
    configuration: PersistedConfiguration,
    credential_versions: HashMap<String, String>,
}

#[derive(Clone)]
struct RuntimeState {
    configuration: PersistedConfiguration,
    revision: u64,
    configuration_revision: u64,
    runtime_plan_revision: u64,
    credential_versions: HashMap<String, String>,
    selected_mode: RuntimeMode,
    desired_mode: RuntimeMode,
    applied_mode: Option<RuntimeMode>,
    phase: RuntimePhase,
    session_health: SessionHealth,
    last_operation: OperationResult,
    session: Option<BackendSession>,
    started_at: Option<Instant>,
    last_error: Option<String>,
    pending: Option<PendingConfiguration>,
    pending_settings: Option<PendingMetadata>,
}

impl RuntimeState {
    fn begin_operation(&mut self) {
        self.last_operation = OperationResult {
            id: self.last_operation.id + 1,
            outcome: OperationOutcome::Pending,
            error: None,
        };
    }

    fn fail_operation(&mut self, error: String) {
        self.last_operation.outcome = OperationOutcome::Failed;
        self.last_operation.error = Some(error.clone());
        self.last_error = Some(error);
    }

    fn complete_operation(&mut self) {
        self.last_operation.outcome = OperationOutcome::Succeeded;
        self.last_operation.error = None;
        if matches!(
            self.session_health,
            SessionHealth::Healthy | SessionHealth::Inactive
        ) {
            self.last_error = None;
        }
    }

    fn snapshot(&self) -> RuntimeSnapshot {
        let system_proxy_enabled = self
            .session
            .as_ref()
            .is_some_and(|run| run.system_proxy_enabled);
        RuntimeSnapshot {
            revision: self.revision,
            configuration_revision: self.configuration_revision,
            runtime_plan_revision: self.runtime_plan_revision,
            selected_mode: self.selected_mode,
            desired_mode: self.desired_mode,
            applied_mode: self.applied_mode,
            phase: self.phase,
            session_health: self.session_health,
            last_operation: self.last_operation.clone(),
            active_profile_id: self.configuration.active_profile_id.clone(),
            runtime_uptime_ms: self
                .started_at
                .map(|start| start.elapsed().as_millis() as u64),
            system_proxy_enabled,
            tun_enabled: self.session.as_ref().is_some_and(|run| run.tun_enabled),
            coverage: if system_proxy_enabled {
                TrafficCoverage::SystemProxyApps
            } else {
                TrafficCoverage::None
            },
            last_error: self.last_error.clone(),
        }
    }
}

pub trait RuntimeCoordinator: Send + Sync {
    fn snapshot(&self) -> RuntimeSnapshot;
    fn report_configuration_recovery_issue(&self, _: &AppError) {}
    fn active_connections(&self) -> ActiveConnectionsSnapshot {
        ActiveConnectionsSnapshot::degraded()
    }
    fn request_mode(&self, mode: RuntimeMode) -> Result<RuntimeSnapshot, AppError>;
    fn stop(&self) -> Result<RuntimeSnapshot, AppError>;
    fn recover_network(&self) -> Result<RuntimeSnapshot, AppError> {
        Err(AppError::unavailable("当前平台不支持系统代理恢复"))
    }
    // An error must leave the previously applied runtime intact.
    fn apply_configuration(
        &self,
        previous: &PersistedConfiguration,
        candidate: &PersistedConfiguration,
    ) -> Result<RuntimeSnapshot, AppError>;
    fn apply_configuration_with_credentials(
        &self,
        previous: &PersistedConfiguration,
        candidate: &PersistedConfiguration,
        _changed_credentials: &[String],
    ) -> Result<RuntimeSnapshot, AppError> {
        self.apply_configuration(previous, candidate)
    }
    fn confirm_configuration(&self) -> RuntimeSnapshot;
    fn restore_configuration(
        &self,
        previous: &PersistedConfiguration,
        snapshot: &RuntimeSnapshot,
    ) -> Result<RuntimeSnapshot, AppError>;
}

#[path = "runtime_manager.rs"]
mod manager;
pub use manager::ManagedRuntime;
