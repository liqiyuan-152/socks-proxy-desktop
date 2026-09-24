use crate::{
    configuration_service::{ConfigurationService, ProfileInput, ProfileView},
    credentials::CredentialUpdate,
    error::AppError,
    models::{AppSettings, RoutingRule, RuntimeMode},
    observability::ActiveConnectionsSnapshot,
    runtime::RuntimeSnapshot,
    store::{DiagnosticFilter, DiagnosticPage, RuntimeDiagnostic, SqliteConfigurationStore},
};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
use tauri::State;

pub type CommandResult<T> = Result<T, AppError>;
type ServiceState<'a> = State<'a, Arc<ConfigurationService>>;
type StoreState<'a> = State<'a, Arc<SqliteConfigurationStore>>;

#[derive(Serialize)]
pub struct HistoryUnavailable {
    pub available: bool,
    pub reason: &'static str,
}

#[derive(Serialize)]
pub struct NetworkRecoveryResult {
    pub completed_at_ms: i64,
    pub snapshot: RuntimeSnapshot,
}

async fn run_blocking<T: Send + 'static>(
    action: impl FnOnce() -> CommandResult<T> + Send + 'static,
) -> CommandResult<T> {
    tauri::async_runtime::spawn_blocking(action)
        .await
        .map_err(|_| AppError::storage("配置操作未能完成"))?
}

#[tauri::command]
pub async fn list_profiles(service: ServiceState<'_>) -> CommandResult<Vec<ProfileView>> {
    let service = Arc::clone(&service);
    run_blocking(move || service.list_profiles()).await
}

#[tauri::command]
pub async fn save_profile(
    service: ServiceState<'_>,
    input: ProfileInput,
) -> CommandResult<ProfileView> {
    let service = Arc::clone(&service);
    run_blocking(move || service.save_profile(input)).await
}

#[tauri::command]
pub async fn delete_profile(service: ServiceState<'_>, id: String) -> CommandResult<()> {
    let service = Arc::clone(&service);
    run_blocking(move || service.delete_profile(&id)).await
}

#[tauri::command]
pub async fn select_profile(service: ServiceState<'_>, id: Option<String>) -> CommandResult<()> {
    let service = Arc::clone(&service);
    run_blocking(move || service.select_profile(id)).await
}

#[tauri::command]
pub async fn list_rules(service: ServiceState<'_>) -> CommandResult<Vec<RoutingRule>> {
    let service = Arc::clone(&service);
    run_blocking(move || service.list_rules()).await
}

#[tauri::command]
pub async fn replace_rules(
    service: ServiceState<'_>,
    rules: Vec<RoutingRule>,
) -> CommandResult<()> {
    let service = Arc::clone(&service);
    run_blocking(move || service.replace_rules(rules)).await
}

#[tauri::command]
pub async fn reorder_rules(service: ServiceState<'_>, ids: Vec<String>) -> CommandResult<()> {
    let service = Arc::clone(&service);
    run_blocking(move || service.reorder_rules(&ids)).await
}

#[tauri::command]
pub async fn get_settings(service: ServiceState<'_>) -> CommandResult<AppSettings> {
    let service = Arc::clone(&service);
    run_blocking(move || service.settings()).await
}

#[tauri::command]
pub async fn update_settings(
    service: ServiceState<'_>,
    settings: AppSettings,
) -> CommandResult<AppSettings> {
    let service = Arc::clone(&service);
    run_blocking(move || service.update_settings(settings)).await
}

#[tauri::command]
pub async fn export_configuration(service: ServiceState<'_>) -> CommandResult<String> {
    let service = Arc::clone(&service);
    run_blocking(move || service.export()).await
}

#[tauri::command]
pub async fn import_configuration(
    service: ServiceState<'_>,
    json: String,
    updates: HashMap<String, CredentialUpdate>,
) -> CommandResult<()> {
    let service = Arc::clone(&service);
    run_blocking(move || service.import(&json, updates)).await
}

#[tauri::command]
pub async fn get_runtime_snapshot(service: ServiceState<'_>) -> CommandResult<RuntimeSnapshot> {
    let service = Arc::clone(&service);
    run_blocking(move || Ok(service.runtime_snapshot())).await
}

#[tauri::command]
pub async fn set_runtime_mode(
    service: ServiceState<'_>,
    mode: RuntimeMode,
) -> CommandResult<RuntimeSnapshot> {
    let service = Arc::clone(&service);
    run_blocking(move || service.request_mode(mode)).await
}

#[tauri::command]
pub async fn stop_runtime(service: ServiceState<'_>) -> CommandResult<RuntimeSnapshot> {
    let service = Arc::clone(&service);
    run_blocking(move || service.stop_runtime()).await
}

#[tauri::command]
pub async fn recover_network(
    service: ServiceState<'_>,
    store: StoreState<'_>,
    confirmed: bool,
) -> CommandResult<NetworkRecoveryResult> {
    if !confirmed {
        return Err(AppError::unavailable("请先确认恢复本应用管理的系统代理"));
    }
    let service = Arc::clone(&service);
    let store = Arc::clone(&store);
    run_blocking(move || {
        let result = service.recover_network();
        let completed_at_ms = crate::store::diagnostic_now_ms()?;
        let diagnostic = RuntimeDiagnostic {
            id: uuid::Uuid::new_v4().to_string(),
            created_at_ms: completed_at_ms,
            severity: if result.is_ok() { "info" } else { "error" }.into(),
            summary: if result.is_ok() {
                "用户确认后已检查并恢复本应用可确认拥有的系统代理设置"
            } else {
                "用户触发的网络恢复未完成，请查看当前运行时状态"
            }
            .into(),
        };
        let recorded = store.record_diagnostic(&diagnostic);
        match result {
            Ok(snapshot) => {
                recorded?;
                Ok(NetworkRecoveryResult {
                    completed_at_ms,
                    snapshot,
                })
            }
            Err(error) => Err(error),
        }
    })
    .await
}

#[tauri::command]
pub async fn get_active_connections(
    service: ServiceState<'_>,
) -> CommandResult<ActiveConnectionsSnapshot> {
    let service = Arc::clone(&service);
    run_blocking(move || Ok(service.active_connections())).await
}

#[tauri::command]
pub async fn copy_active_connection_detail(
    service: ServiceState<'_>,
    id: String,
) -> CommandResult<String> {
    let service = Arc::clone(&service);
    run_blocking(move || service.active_connections().copy_detail(&id)).await
}

#[tauri::command]
pub async fn get_runtime_diagnostics(
    store: StoreState<'_>,
    filter: DiagnosticFilter,
    offset: usize,
    limit: usize,
) -> CommandResult<DiagnosticPage> {
    let store = Arc::clone(&store);
    run_blocking(move || store.list_diagnostics(&filter, offset, limit)).await
}

#[tauri::command]
pub async fn clear_runtime_diagnostics(
    store: StoreState<'_>,
    filter: DiagnosticFilter,
    confirmed: bool,
) -> CommandResult<usize> {
    let store = Arc::clone(&store);
    run_blocking(move || store.clear_diagnostics(&filter, confirmed)).await
}

#[tauri::command]
pub fn get_connection_history(
    _filter: Option<serde_json::Value>,
    _cursor: Option<String>,
) -> HistoryUnavailable {
    HistoryUnavailable {
        available: false,
        reason: "当前内核不提供可验证的已完成连接结果或历史记录",
    }
}

#[cfg(test)]
#[path = "ipc_tests.rs"]
mod tests;
