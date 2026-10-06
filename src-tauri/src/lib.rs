#[cfg(test)]
mod application_latency_lifecycle_tests;
mod capabilities;
mod china_rules;
#[cfg(test)]
mod china_rules_tests;
#[cfg(test)]
mod configuration_crash_tests;
mod configuration_document;
mod configuration_recovery;
mod configuration_startup_recovery;
#[cfg(any(windows, test))]
mod core_control;
mod credentials;
pub mod domain_errors;
mod error;
pub mod error_context;
mod ipc;
#[cfg(test)]
mod ipc_contract;
mod ipc_latency;
#[cfg(test)]
mod ipc_latency_tests;
#[cfg(any(windows, test))]
mod latency;
mod latency_tasks;
mod models;
mod observability;
pub mod performance_metrics;
mod route_test;
mod routing;
mod rule_match_process;
mod runtime;
#[cfg(all(test, windows))]
mod runtime_crash_tests;
mod runtime_events;
mod runtime_plan;
mod runtime_session;
#[cfg(test)]
mod runtime_soak_tests;
pub mod runtime_state_machine;
#[cfg(not(windows))]
mod runtime_unavailable;
pub mod services;
#[cfg(any(windows, test))]
mod sing_box_backend;
#[cfg(any(windows, test))]
mod sing_box_config;
#[cfg(any(windows, test))]
mod sing_box_process;
#[cfg(windows)]
mod sing_box_windows_acl;
#[cfg(windows)]
mod sing_box_windows_job;
mod startup;
mod startup_metrics;
mod store;
#[cfg(any(windows, test))]
mod system_proxy;
#[cfg(windows)]
mod system_proxy_windows;
#[cfg(test)]
mod test_core;
mod trace_writer;
mod tracing_setup;
mod transfer;
mod tray;

use credentials::OsCredentialStore;
use runtime::RuntimeBackend;
pub use runtime::{ManagedRuntime, TransitionRecord};
use runtime_session::{SessionLease, RUNTIME_SESSION_KEY};
#[cfg(not(windows))]
use runtime_unavailable::UnavailableRuntimeBackend;
use services::ApplicationService;
use startup::SystemStartupAdapter;
use std::sync::Arc;
use store::{ConfigurationStore, SqliteConfigurationStore};
use tauri::Manager;

#[cfg(windows)]
type WindowsBackendResult =
    Result<(Box<dyn RuntimeBackend>, Option<String>), Box<dyn std::error::Error>>;

#[cfg(windows)]
fn windows_backend<R: tauri::Runtime>(
    app: &tauri::App<R>,
    store: Arc<SqliteConfigurationStore>,
    app_data: &std::path::Path,
) -> WindowsBackendResult {
    use sing_box_backend::SingBoxRuntimeBackend;
    use sing_box_process::WINDOWS_AMD64_EXE_SHA256;
    use system_proxy::OwnedSystemProxy;
    use system_proxy_windows::WindowsProxyDevice;
    use tauri::path::BaseDirectory;

    let proxy = OwnedSystemProxy::new(Box::<WindowsProxyDevice>::default(), Box::new(store));
    // Recovery precedes any new core or proxy mutation. An unresolved journal
    // must never be silently overwritten by a new runtime session.
    let recovery_issue = proxy.recover_on_startup().err().map(|error| error.message);
    // No other instance can own these runs: the session lease was acquired first.
    sing_box_process::cleanup_stale_runtime_dirs(&app_data.join("runtime"))?;
    let binary = app.path().resolve(
        "sing-box/windows-amd64/sing-box.exe",
        BaseDirectory::Resource,
    )?;
    Ok((
        Box::new(SingBoxRuntimeBackend::new(
            binary,
            WINDOWS_AMD64_EXE_SHA256.into(),
            app_data.join("runtime"),
            Arc::new(OsCredentialStore),
            Box::new(proxy),
        )),
        recovery_issue,
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let bootstrap_started = std::time::Instant::now();
    tauri::Builder::default()
        .manage(startup_metrics::StartupMetrics::new(bootstrap_started))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            capabilities::get_capabilities,
            startup_metrics::acknowledge_frontend_ready,
            ipc::list_profiles,
            ipc::get_profile_credential,
            ipc_latency::test_proxy_latency,
            ipc_latency::start_proxy_latency_task,
            ipc_latency::get_proxy_latency_task,
            ipc_latency::release_proxy_latency_task,
            ipc::save_profile,
            ipc::delete_profile,
            ipc::select_profile,
            ipc::list_rules,
            ipc::replace_rules,
            ipc::reorder_rules,
            ipc::get_settings,
            ipc::get_china_direct_status,
            ipc::set_china_direct_enabled,
            ipc::test_route,
            ipc::update_settings,
            ipc::export_configuration,
            ipc::save_configuration,
            ipc::import_configuration,
            ipc::get_runtime_snapshot,
            ipc::export_runtime_snapshot,
            ipc::validate_configuration,
            ipc::set_runtime_mode,
            ipc::stop_runtime,
            ipc::recover_network,
            ipc::get_active_connections,
            ipc::copy_active_connection_detail,
            ipc::get_runtime_diagnostics,
            ipc::get_diagnostic_groups,
            ipc::export_runtime_diagnostics,
            ipc::save_runtime_diagnostics,
            ipc::clear_runtime_diagnostics,
            ipc::get_connection_history,
        ])
        .setup(move |app| {
            // Claim the per-session runtime before opening or migrating any state.
            let ownership = SessionLease::acquire(RUNTIME_SESSION_KEY)?;
            let app_data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data)?;
            app.manage(tracing_setup::initialize(&app_data.join("logs")));
            let store = Arc::new(SqliteConfigurationStore::open(
                app_data.join("config.sqlite3"),
            )?);
            let selected_mode = store.load_mode()?;
            #[cfg(windows)]
            let (backend, recovery_issue) = windows_backend(app, store.clone(), &app_data)?;
            #[cfg(not(windows))]
            let backend: Box<dyn RuntimeBackend> = Box::new(UnavailableRuntimeBackend);
            #[cfg(not(windows))]
            let recovery_issue: Option<String> = None;
            let recovery_issue = recovery_issue.or_else(|| {
                configuration_startup_recovery::recover_configuration_on_startup(
                    &ownership,
                    store.as_ref(),
                    &OsCredentialStore,
                    &SystemStartupAdapter,
                )
                .err()
                .map(|error| error.message)
            });
            let initial = store.load()?;
            let runtime = ManagedRuntime::from_lease(initial, backend, ownership, selected_mode)?
                .with_configuration_revision(store.recovery_revision()?);
            let recovery_pending = recovery_issue.is_some();
            if let Some(message) = recovery_issue {
                runtime.report_startup_recovery_issue(message);
            }
            let runtime_updates = runtime.subscribe();
            let service = ApplicationService::new(
                Box::new(store.clone()),
                Box::new(OsCredentialStore),
                Box::new(SystemStartupAdapter),
                Box::new(runtime),
            )
            .with_startup_recovery_pending(recovery_pending);
            #[cfg(windows)]
            let service = service.with_china_rule_root(
                app.path()
                    .resolve("china-rules", tauri::path::BaseDirectory::Resource)?,
            );
            if !recovery_pending {
                if let Err(error) = service.upgrade_legacy_credentials() {
                    service.report_configuration_recovery_issue(&error);
                }
            }
            #[cfg(windows)]
            let service = {
                use tauri::path::BaseDirectory;
                let binary = app.path().resolve(
                    "sing-box/windows-amd64/sing-box.exe",
                    BaseDirectory::Resource,
                )?;
                let executor = Arc::new(latency::LatencyTester::for_runtime(
                    binary,
                    sing_box_process::WINDOWS_AMD64_EXE_SHA256.into(),
                    app_data.join("runtime"),
                ));
                service.with_latency_tasks(latency_tasks::LatencyTaskRegistry::new(executor))
            };
            app.manage(Arc::new(service));
            app.manage(store.clone());

            tray::install(app)?;
            runtime_events::start(app.handle().clone(), store, runtime_updates);

            performance_metrics::record(
                "startup_bootstrap",
                bootstrap_started.elapsed().as_secs_f64() * 1000.0,
                true,
            );
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
