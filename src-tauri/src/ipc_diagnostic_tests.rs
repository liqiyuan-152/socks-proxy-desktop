use super::*;
use crate::{
    services::application_service::tests::{FakeRuntime, MemoryCredentials, MemoryStartup},
    store::{ConfigurationStore, RuntimeDiagnostic},
};
use serde_json::{json, Value};
use tauri::{
    ipc::{CallbackFn, InvokeBody},
    test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY},
    webview::InvokeRequest,
    WebviewWindowBuilder,
};

#[test]
fn diagnostic_commands_export_full_json_lines_and_aggregate_typed_failures() -> Result<(), AppError>
{
    let store = Arc::new(SqliteConfigurationStore::open_in_memory()?);
    let at = crate::store::diagnostic_now_ms()?;
    for index in 0..105 {
        store.record_diagnostic(&RuntimeDiagnostic {
            id: format!("error-{index}"),
            created_at_ms: at + index,
            severity: "warning".into(),
            summary: "应用操作失败".into(),
            error_type: Some("proxy.not_found".into()),
            operation: Some("delete_profile".into()),
        })?;
    }
    let service = Arc::new(ApplicationService::new(
        Box::new(store),
        Box::new(Arc::new(MemoryCredentials::default())),
        Box::new(Arc::new(MemoryStartup::default())),
        Box::new(Arc::new(FakeRuntime::default())),
    ));
    let app = mock_builder()
        .manage(service)
        .invoke_handler(tauri::generate_handler![
            get_diagnostic_groups,
            export_runtime_diagnostics,
            save_runtime_diagnostics
        ])
        .build(mock_context(noop_assets()))
        .expect("test app");
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("test webview");
    let invoke = |command: &str, filter: Value| -> Result<Value, Value> {
        get_ipc_response(
            &window,
            InvokeRequest {
                cmd: command.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: window.url().expect("local webview URL"),
                body: InvokeBody::Json(json!({"filter": filter})),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.into(),
            },
        )
        .map(|body| body.deserialize::<Value>().expect("response JSON"))
    };
    let filter = json!({"from_ms": null, "until_ms": null, "severity": "warning"});
    let groups = invoke("get_diagnostic_groups", filter.clone()).expect("aggregate response");
    assert_eq!(groups[0]["error_type"], "proxy.not_found");
    assert_eq!(groups[0]["occurrences"], 105);
    let export = invoke("export_runtime_diagnostics", filter).expect("export response");
    let lines = export
        .as_str()
        .expect("JSON Lines string")
        .lines()
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 105);
    let first: Value = serde_json::from_str(lines[0]).expect("JSON line");
    assert_eq!(first["id"], "error-104");
    assert_eq!(first["operation"], "delete_profile");
    let failure = invoke(
        "export_runtime_diagnostics",
        json!({"from_ms": null, "until_ms": null, "severity": "invalid"}),
    )
    .expect_err("invalid filter");
    assert_eq!(
        failure["context"]["operation"],
        "export_runtime_diagnostics"
    );
    let rejected_save = invoke(
        "save_runtime_diagnostics",
        json!({"from_ms": null, "until_ms": null, "severity": "invalid"}),
    )
    .expect_err("invalid filter is rejected before opening a dialog");
    assert_eq!(rejected_save["code"], "unavailable");

    Ok(())
}
