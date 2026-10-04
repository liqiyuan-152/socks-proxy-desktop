use super::*;
use crate::services::application_service::tests::Fixture;
use serde_json::{json, Value};
use tauri::{
    ipc::{CallbackFn, InvokeBody},
    test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY},
    webview::InvokeRequest,
    WebviewWindowBuilder,
};

#[test]
fn runtime_tools_are_callable_through_tauri_without_configuration_or_credentials(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new();
    let app = mock_builder()
        .manage(Arc::new(fixture.service))
        .invoke_handler(tauri::generate_handler![
            export_runtime_snapshot,
            validate_configuration
        ])
        .build(mock_context(noop_assets()))?;
    let window = WebviewWindowBuilder::new(&app, "main", Default::default()).build()?;
    for command in ["export_runtime_snapshot", "validate_configuration"] {
        let response = get_ipc_response(
            &window,
            InvokeRequest {
                cmd: command.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: window.url()?,
                body: InvokeBody::Json(json!({})),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.into(),
            },
        )
        .expect("tool succeeds")
        .deserialize::<String>()?;
        let report: Value = serde_json::from_str(&response)?;
        assert!(report["runtime"].is_object());
        assert!(!response.contains("password"));
        assert!(!response.contains("credential"));
        if command == "validate_configuration" {
            assert_eq!(report["valid"], true);
        }
    }
    Ok(())
}
