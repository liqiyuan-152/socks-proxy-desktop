use super::*;
use crate::{
    credentials::{CredentialStore, ProxyCredential},
    models::{PersistedConfiguration, RuntimeMode},
    runtime::{BackendSession, ManagedRuntime, RuntimeBackend},
    runtime_session::SessionLease,
    startup::StartupAdapter,
    store::{ConfigurationStore, SqliteConfigurationStore},
};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    ipc::{CallbackFn, InvokeBody},
    test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY},
    webview::InvokeRequest,
    WebviewWindowBuilder,
};

struct Credentials;
impl CredentialStore for Credentials {
    fn get(&self, _: &str) -> CommandResult<Option<ProxyCredential>> {
        Ok(None)
    }
    fn replace(&self, _: &str, _: &str, _: &str) -> CommandResult<()> {
        Ok(())
    }
    fn delete(&self, _: &str) -> CommandResult<()> {
        Ok(())
    }
}

struct Startup(AtomicBool);
impl StartupAdapter for Arc<Startup> {
    fn is_enabled(&self) -> CommandResult<bool> {
        Ok(self.0.load(Ordering::SeqCst))
    }
    fn set_enabled(&self, enabled: bool) -> CommandResult<()> {
        self.0.store(enabled, Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Default)]
struct Backend {
    reject_stop: AtomicBool,
    reject_mode: AtomicBool,
    exited: AtomicBool,
}
impl RuntimeBackend for Arc<Backend> {
    fn restore_network(&self) -> CommandResult<()> {
        Ok(())
    }
    fn transition(
        &self,
        _: Option<&BackendSession>,
        _: &PersistedConfiguration,
        mode: RuntimeMode,
        revision: u64,
    ) -> CommandResult<Option<BackendSession>> {
        if self.reject_mode.swap(false, Ordering::SeqCst) {
            return Err(AppError::unavailable("模拟内核候选启动失败"));
        }
        if mode == RuntimeMode::Direct {
            if self.reject_stop.load(Ordering::SeqCst) {
                return Err(AppError::unavailable("模拟系统代理恢复失败"));
            }
            return Ok(None);
        }
        Ok(Some(BackendSession {
            run_id: format!("run-{revision}"),
            process_id: 42,
            configuration_revision: revision,
            system_proxy_enabled: true,
            tun_enabled: false,
        }))
    }

    fn confirm_transition(&self, _: Option<&BackendSession>) {}

    fn revert_transition(
        &self,
        _: Option<&BackendSession>,
        _: Option<&BackendSession>,
    ) -> CommandResult<()> {
        Ok(())
    }

    fn reconcile_session(&self, _: &BackendSession) -> CommandResult<bool> {
        Ok(!self.exited.load(Ordering::SeqCst))
    }
}

#[test]
fn actual_tauri_commands_validate_and_persist_configuration() {
    let store = Arc::new(SqliteConfigurationStore::open_in_memory().unwrap());
    let initial = crate::models::PersistedConfiguration::default();
    let backend = Arc::new(Backend::default());
    let runtime = ManagedRuntime::from_lease(
        initial,
        Box::new(backend.clone()),
        SessionLease::acquire(&uuid::Uuid::new_v4().to_string()).unwrap(),
        RuntimeMode::Direct,
    )
    .unwrap();
    let startup = Arc::new(Startup(AtomicBool::new(false)));
    let service = Arc::new(ConfigurationService::new(
        Box::new(store.clone()),
        Box::new(Credentials),
        Box::new(startup.clone()),
        Box::new(runtime),
    ));
    let app = mock_builder()
        .manage(service)
        .manage(store.clone())
        .invoke_handler(tauri::generate_handler![
            list_profiles,
            get_profile_credential,
            save_profile,
            delete_profile,
            select_profile,
            list_rules,
            replace_rules,
            reorder_rules,
            get_settings,
            update_settings,
            export_configuration,
            import_configuration,
            get_runtime_snapshot,
            set_runtime_mode,
            stop_runtime,
            recover_network,
            get_active_connections,
            copy_active_connection_detail,
            get_runtime_diagnostics,
            clear_runtime_diagnostics,
            get_connection_history,
        ])
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let invoke = |cmd: &str, body: Value| -> Result<Value, Value> {
        get_ipc_response(
            &webview,
            InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                // Windows serves Tauri's local origin over http rather than
                // the tauri:// custom scheme used on other platforms.
                url: webview.url().unwrap(),
                body: InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
        .map(|body| body.deserialize().unwrap())
    };

    let input = json!({
        "name": "Primary", "protocol": "socks5", "host": "proxy.example.com",
        "port": 1080, "authentication_enabled": false, "enabled": true,
    });
    let created = invoke("save_profile", json!({ "input": input })).unwrap();
    let id = created["id"].as_str().unwrap();
    assert_eq!(created["name"], "Primary");
    assert!(created.get("password").is_none());
    assert_eq!(
        invoke("get_profile_credential", json!({ "id": id })).unwrap_err()["code"],
        "unavailable"
    );
    let invalid = invoke(
        "save_profile",
        json!({ "input": {
        "name": "Second", "protocol": "http", "host": "", "port": 0,
        "authentication_enabled": false, "enabled": true,
    } }),
    )
    .unwrap_err();
    assert_eq!(invalid["code"], "validation_error");
    assert_eq!(invalid["fields"][0]["field"], "profiles[1].host");
    assert_eq!(
        invoke("list_profiles", json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let first = json!({ "id": "first", "name": "First", "matcher": "domain_suffix",
        "target": "example.com", "port_start": null, "port_end": null,
        "action": "direct", "enabled": true });
    let second = json!({ "id": "second", "name": "Second", "matcher": "domain",
        "target": "api.example.com", "port_start": null, "port_end": null,
        "action": "proxy", "proxy_profile_id": id, "enabled": true });
    invoke("replace_rules", json!({ "rules": [first, second] })).unwrap();
    invoke("reorder_rules", json!({ "ids": ["second", "first"] })).unwrap();
    assert_eq!(invoke("list_rules", json!({})).unwrap()[0]["id"], "second");
    let conflict = invoke("reorder_rules", json!({ "ids": ["first", "first"] })).unwrap_err();
    assert_eq!(conflict["fields"][0]["field"], "rule_ids");

    invoke("select_profile", json!({ "id": id })).unwrap();
    let settings = json!({ "launch_at_login": true, "diagnostic_retention": "days7" });
    invoke("update_settings", json!({ "settings": settings })).unwrap();
    assert_eq!(
        invoke("get_settings", json!({})).unwrap()["launch_at_login"],
        true
    );
    let exported = invoke("export_configuration", json!({})).unwrap();
    let json = exported.as_str().unwrap();
    assert!(!json.contains("password"));
    invoke(
        "import_configuration",
        json!({ "json": json, "updates": {} }),
    )
    .unwrap();
    let running = invoke("set_runtime_mode", json!({ "mode": "global" })).unwrap();
    assert_eq!(running["applied_mode"], "global");
    backend.reject_mode.store(true, Ordering::SeqCst);
    assert_eq!(
        invoke("set_runtime_mode", json!({ "mode": "rules" })).unwrap_err()["code"],
        "unavailable"
    );
    let rolled_back = invoke("get_runtime_snapshot", json!({})).unwrap();
    assert_eq!(rolled_back["applied_mode"], "global");
    assert_eq!(rolled_back["desired_mode"], "rules");
    assert_eq!(rolled_back["selected_mode"], "rules");
    assert_eq!(store.load_mode().unwrap(), RuntimeMode::Rules);
    let error_filter = json!({"from_ms": null, "until_ms": null, "severity": "error"});
    let failure = invoke(
        "get_runtime_diagnostics",
        json!({"filter": error_filter, "offset": 0, "limit": 10}),
    )
    .unwrap();
    assert_eq!(failure["total"], 1);
    assert!(failure["items"][0]["summary"]
        .as_str()
        .unwrap()
        .contains("切换至规则代理失败"));
    backend.reject_stop.store(true, Ordering::SeqCst);
    let failed = invoke("delete_profile", json!({ "id": id })).unwrap_err();
    assert_eq!(failed["code"], "validation_error");
    assert_eq!(failed["fields"][0]["field"], "rules[0].proxy_profile_id");
    assert_eq!(
        invoke("get_runtime_snapshot", json!({})).unwrap()["applied_mode"],
        "global"
    );
    assert_eq!(
        invoke("list_profiles", json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    backend.reject_stop.store(false, Ordering::SeqCst);
    invoke("replace_rules", json!({ "rules": [] })).unwrap();
    let no_default = invoke("select_profile", json!({ "id": null })).unwrap_err();
    assert_eq!(no_default["fields"][0]["field"], "default_profile_id");
    invoke("set_runtime_mode", json!({ "mode": "direct" })).unwrap();
    invoke("select_profile", json!({ "id": null })).unwrap();
    invoke("delete_profile", json!({ "id": id })).unwrap();
    assert_eq!(
        invoke("get_runtime_snapshot", json!({})).unwrap()["applied_mode"],
        "direct"
    );
    assert!(invoke("list_profiles", json!({}))
        .unwrap()
        .as_array()
        .unwrap()
        .is_empty());

    let history = invoke(
        "get_connection_history",
        json!({ "filter": {}, "cursor": null }),
    )
    .unwrap();
    assert_eq!(history["available"], false);
    assert_eq!(
        invoke("recover_network", json!({ "confirmed": false })).unwrap_err()["code"],
        "unavailable"
    );
    let recovery = invoke("recover_network", json!({ "confirmed": true })).unwrap();
    assert!(recovery["completed_at_ms"].as_i64().unwrap() > 0);
    assert_eq!(recovery["snapshot"]["phase"], "stopped");
    assert!(history.get("items").is_none());
    let inactive = invoke("get_active_connections", json!({})).unwrap();
    assert_eq!(inactive["status"], "degraded");
    assert_eq!(inactive["active_count"], Value::Null);
    let filter = json!({"from_ms": null, "until_ms": null, "severity": null});
    assert_eq!(
        invoke(
            "get_runtime_diagnostics",
            json!({"filter": filter, "offset": 0, "limit": 10})
        )
        .unwrap()["total"],
        2
    );
    assert_eq!(
        invoke(
            "clear_runtime_diagnostics",
            json!({"filter": filter, "confirmed": false})
        )
        .unwrap_err()["code"],
        "unavailable"
    );
    store
        .record_diagnostic(&crate::store::RuntimeDiagnostic {
            id: "restart-event".into(),
            created_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            severity: "info".into(),
            summary: "运行时启动".into(),
        })
        .unwrap();
    assert_eq!(
        invoke(
            "get_runtime_diagnostics",
            json!({"filter": filter, "offset": 0, "limit": 10})
        )
        .unwrap()["total"],
        3
    );
    assert_eq!(
        invoke(
            "clear_runtime_diagnostics",
            json!({"filter": filter, "confirmed": true})
        )
        .unwrap(),
        3
    );

    let replacement = invoke(
        "save_profile",
        json!({"input": {
            "name": "Replacement", "protocol": "http", "host": "proxy.example.org",
            "port": 8080, "authentication_enabled": false, "enabled": true
        }}),
    )
    .unwrap();
    invoke("select_profile", json!({"id": replacement["id"]})).unwrap();
    assert_eq!(
        invoke("set_runtime_mode", json!({"mode": "global"})).unwrap()["phase"],
        "running"
    );
    backend.exited.store(true, Ordering::SeqCst);
    let crashed = invoke("get_runtime_snapshot", json!({})).unwrap();
    assert_eq!(crashed["phase"], "failed");
    assert_eq!(crashed["applied_mode"], Value::Null);
    assert_eq!(
        invoke("get_active_connections", json!({})).unwrap()["status"],
        "degraded"
    );
}
