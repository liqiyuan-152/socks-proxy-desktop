use crate::{
    error::AppError,
    models::{PersistedConfiguration, RuntimeMode},
    observability::ActiveConnectionsSnapshot,
};
use serde::Serialize;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
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
pub enum TrafficCoverage {
    None,
    SystemProxyApps,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeSnapshot {
    pub revision: u64,
    pub desired_mode: RuntimeMode,
    pub applied_mode: Option<RuntimeMode>,
    pub phase: RuntimePhase,
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
}

#[derive(Clone)]
struct RuntimeState {
    configuration: PersistedConfiguration,
    revision: u64,
    desired_mode: RuntimeMode,
    applied_mode: Option<RuntimeMode>,
    phase: RuntimePhase,
    session: Option<BackendSession>,
    started_at: Option<Instant>,
    last_error: Option<String>,
    pending: Option<PendingConfiguration>,
}

impl RuntimeState {
    fn snapshot(&self) -> RuntimeSnapshot {
        let system_proxy_enabled = self
            .session
            .as_ref()
            .is_some_and(|run| run.system_proxy_enabled);
        RuntimeSnapshot {
            revision: self.revision,
            desired_mode: self.desired_mode,
            applied_mode: self.applied_mode,
            phase: self.phase,
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
