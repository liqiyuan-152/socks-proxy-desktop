use crate::{
    credentials::CredentialUpdate,
    error::AppError,
    models::{AppSettings, RoutingRule, RuntimeMode},
    observability::ActiveConnectionsSnapshot,
    route_test::RouteTestResult,
    runtime::RuntimeSnapshot,
    services::{
        ApplicationService, ChinaDirectStatus, ProfileCredentialView, ProfileInput, ProfileView,
    },
    store::{DiagnosticFilter, DiagnosticGroup, DiagnosticPage, SqliteConfigurationStore},
};
use std::{collections::HashMap, sync::Arc};
use tauri::State;

pub type CommandResult<T> = Result<T, AppError>;
type ServiceState<'a> = State<'a, Arc<ApplicationService>>;
type StoreState<'a> = State<'a, Arc<SqliteConfigurationStore>>;

pub use crate::services::{HistoryUnavailable, NetworkRecoveryResult};

pub(crate) async fn run_blocking<T: Send + 'static>(
    action: impl FnOnce() -> CommandResult<T> + Send + 'static,
) -> CommandResult<T> {
    tauri::async_runtime::spawn_blocking(action)
        .await
        .map_err(|_| {
            AppError::storage("配置操作未能完成").with_context(
                crate::error_context::ErrorDomain::Application,
                "worker_failed",
            )
        })?
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
pub async fn get_china_direct_status(
    service: ServiceState<'_>,
) -> CommandResult<ChinaDirectStatus> {
    let service = Arc::clone(&service);
    run_blocking(move || service.china_direct_status()).await
}

#[tauri::command]
pub async fn set_china_direct_enabled(
    service: ServiceState<'_>,
    enabled: bool,
) -> CommandResult<ChinaDirectStatus> {
    let service = Arc::clone(&service);
    run_blocking(move || service.set_china_direct_enabled(enabled)).await
}

#[tauri::command]
pub async fn test_route(
    service: ServiceState<'_>,
    target: String,
    port: u16,
) -> CommandResult<RouteTestResult> {
    let service = Arc::clone(&service);
    run_blocking(move || service.test_route(&target, port)).await
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
    _store: StoreState<'_>,
    mode: RuntimeMode,
) -> CommandResult<RuntimeSnapshot> {
    let service = Arc::clone(&service);
    run_blocking(move || service.request_mode_with_diagnostics(mode)).await
}

#[tauri::command]
pub async fn get_profile_credential(
    service: ServiceState<'_>,
    id: String,
) -> CommandResult<ProfileCredentialView> {
    let service = Arc::clone(&service);
    run_blocking(move || service.profile_credential(&id)).await
}

#[tauri::command]
pub async fn stop_runtime(service: ServiceState<'_>) -> CommandResult<RuntimeSnapshot> {
    let service = Arc::clone(&service);
    run_blocking(move || service.stop_runtime()).await
}

#[tauri::command]
pub async fn recover_network(
    service: ServiceState<'_>,
    _store: StoreState<'_>,
    confirmed: bool,
) -> CommandResult<NetworkRecoveryResult> {
    let service = Arc::clone(&service);
    run_blocking(move || service.recover_network_confirmed(confirmed)).await
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
    run_blocking(move || service.copy_active_connection_detail(&id)).await
}

#[tauri::command]
pub async fn get_runtime_diagnostics(
    service: ServiceState<'_>,
    _store: StoreState<'_>,
    filter: DiagnosticFilter,
    offset: usize,
    limit: usize,
) -> CommandResult<DiagnosticPage> {
    let service = Arc::clone(&service);
    run_blocking(move || service.runtime_diagnostics(&filter, offset, limit)).await
}

#[tauri::command]
pub async fn get_diagnostic_groups(
    service: ServiceState<'_>,
    filter: DiagnosticFilter,
) -> CommandResult<Vec<DiagnosticGroup>> {
    let service = Arc::clone(&service);
    run_blocking(move || service.diagnostic_groups(&filter)).await
}

#[tauri::command]
pub async fn export_runtime_diagnostics(
    service: ServiceState<'_>,
    filter: DiagnosticFilter,
) -> CommandResult<String> {
    let service = Arc::clone(&service);
    run_blocking(move || service.export_runtime_diagnostics(&filter)).await
}

/// 选择位置后保存脱敏配置；取消不写文件，原数据接口保持兼容。
#[tauri::command]
pub async fn save_configuration<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    service: ServiceState<'_>,
) -> CommandResult<bool> {
    use tauri_plugin_dialog::DialogExt;
    let service = Arc::clone(&service);
    run_blocking(move || {
        let content = service.export()?;
        let Some(destination) = app
            .dialog()
            .file()
            .set_file_name("socks-proxy-config.json")
            .add_filter("JSON", &["json"])
            .blocking_save_file()
        else {
            return Ok(false);
        };
        let path = destination
            .into_path()
            .map_err(|_| AppError::storage("请选择本地配置文件保存位置"))?;
        service.save_configuration_file(&path, &content)?;
        Ok(true)
    })
    .await
}

/// 使用原生保存选择器，只能写入用户在本次对话框选择的位置。
#[tauri::command]
pub async fn save_runtime_diagnostics<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    service: ServiceState<'_>,
    filter: DiagnosticFilter,
) -> CommandResult<bool> {
    use tauri_plugin_dialog::DialogExt;
    let service = Arc::clone(&service);
    run_blocking(move || {
        let content = service.export_runtime_diagnostics(&filter)?;
        let Some(destination) = app
            .dialog()
            .file()
            .set_file_name("socks-proxy-diagnostics.jsonl")
            .add_filter("JSON Lines", &["jsonl"])
            .blocking_save_file()
        else {
            return Ok(false);
        };
        let path = destination
            .into_path()
            .map_err(|_| AppError::storage("请选择本地诊断文件保存位置"))?;
        service.save_diagnostics_file(&path, &content)?;
        Ok(true)
    })
    .await
}

#[tauri::command]
pub async fn clear_runtime_diagnostics(
    service: ServiceState<'_>,
    _store: StoreState<'_>,
    filter: DiagnosticFilter,
    confirmed: bool,
) -> CommandResult<usize> {
    let service = Arc::clone(&service);
    run_blocking(move || service.clear_runtime_diagnostics(&filter, confirmed)).await
}

#[tauri::command]
pub fn get_connection_history(
    service: ServiceState<'_>,
    _filter: Option<serde_json::Value>,
    _cursor: Option<String>,
) -> HistoryUnavailable {
    service.connection_history()
}

#[cfg(test)]
#[path = "ipc_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "ipc_diagnostic_tests.rs"]
mod diagnostic_tests;

/// 导出不含配置输入及凭据的运行时诊断快照。
#[tauri::command]
pub async fn export_runtime_snapshot(service: ServiceState<'_>) -> CommandResult<String> {
    let service = Arc::clone(&service);
    run_blocking(move || service.export_runtime_snapshot()).await
}

/// 只读校验持久配置及恢复状态。
#[tauri::command]
pub async fn validate_configuration(service: ServiceState<'_>) -> CommandResult<String> {
    let service = Arc::clone(&service);
    run_blocking(move || service.validate_configuration()).await
}

#[cfg(test)]
#[path = "ipc_runtime_tools_tests.rs"]
mod runtime_tools_tests;
