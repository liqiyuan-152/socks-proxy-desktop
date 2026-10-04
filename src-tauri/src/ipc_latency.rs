use crate::{
    configuration_service::ConfigurationService,
    error::AppError,
    latency_tasks::{LatencyResult, LatencySubscription, LatencyTaskSnapshot},
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn start_proxy_latency_task(
    service: State<'_, Arc<ConfigurationService>>,
    id: String,
) -> Result<LatencySubscription, AppError> {
    let service = Arc::clone(&service);
    crate::ipc::run_blocking(move || service.start_latency_task(&id)).await
}
#[tauri::command]
pub fn get_proxy_latency_task(
    service: State<'_, Arc<ConfigurationService>>,
    subscription_id: String,
) -> Result<LatencyTaskSnapshot, AppError> {
    service.latency_task_snapshot(&subscription_id)
}
#[tauri::command]
pub fn release_proxy_latency_task(
    service: State<'_, Arc<ConfigurationService>>,
    subscription_id: String,
) -> Result<(), AppError> {
    service.release_latency_task(&subscription_id)
}
#[tauri::command]
pub async fn test_proxy_latency(
    service: State<'_, Arc<ConfigurationService>>,
    id: String,
) -> Result<LatencyResult, AppError> {
    let service = Arc::clone(&service);
    crate::ipc::run_blocking(move || service.test_latency_compat(&id)).await
}
