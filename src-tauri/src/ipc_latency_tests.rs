use crate::{
    configuration_service::{ConfigurationService, ProfileInput},
    credentials::{CredentialStore, ProxyCredential},
    error::AppError,
    latency_tasks::{LatencyExecutor, LatencyInput, LatencyResult, LatencyTaskRegistry},
    models::{ProxyProtocol, RuntimeMode},
    runtime::{ManagedRuntime, RuntimeCoordinator},
    runtime_session::SessionLease,
    startup::SystemStartupAdapter,
    store::{ConfigurationStore, SqliteConfigurationStore},
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{
    ipc::{CallbackFn, InvokeBody},
    test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY},
    webview::InvokeRequest,
    WebviewWindowBuilder,
};

struct Credentials;
impl CredentialStore for Credentials {
    fn get(&self, _: &str) -> Result<Option<ProxyCredential>, AppError> {
        Ok(None)
    }
    fn replace(&self, _: &str, _: &str, _: &str) -> Result<(), AppError> {
        panic!("plain profile")
    }
    fn delete(&self, _: &str) -> Result<(), AppError> {
        panic!("plain profile")
    }
}
struct Executor;
impl LatencyExecutor for Executor {
    fn run(
        &self,
        input: LatencyInput,
        cancelled: Arc<AtomicBool>,
    ) -> Result<LatencyResult, AppError> {
        assert!(!input.profile_id.is_empty());
        assert!(input.credential.is_none());
        while !cancelled.load(Ordering::Acquire) {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        Ok(LatencyResult { latency_ms: 999 })
    }
}
struct Backend;
impl crate::runtime::RuntimeBackend for Backend {
    fn transition(
        &self,
        _: Option<&crate::runtime::BackendSession>,
        _: &crate::models::PersistedConfiguration,
        _: RuntimeMode,
        _: u64,
    ) -> Result<Option<crate::runtime::BackendSession>, AppError> {
        Ok(None)
    }
    fn confirm_transition(&self, _: Option<&crate::runtime::BackendSession>) {}
    fn revert_transition(
        &self,
        _: Option<&crate::runtime::BackendSession>,
        _: Option<&crate::runtime::BackendSession>,
    ) -> Result<(), AppError> {
        Ok(())
    }
    fn reconcile_session(&self, _: &crate::runtime::BackendSession) -> Result<bool, AppError> {
        Ok(false)
    }
}
#[test]
fn actual_task_ipc_subscribes_reuses_reports_and_releases() {
    let store = Arc::new(SqliteConfigurationStore::open_in_memory().unwrap());
    let runtime = ManagedRuntime::from_lease(
        store.load().unwrap(),
        Box::new(Backend),
        SessionLease::acquire(&uuid::Uuid::new_v4().to_string()).unwrap(),
        RuntimeMode::Direct,
    )
    .unwrap();
    assert_eq!(runtime.snapshot().configuration_revision, 0);
    let service = Arc::new(
        ConfigurationService::new(
            Box::new(store),
            Box::new(Credentials),
            Box::new(SystemStartupAdapter),
            Box::new(runtime),
        )
        .with_latency_tasks(LatencyTaskRegistry::new(Arc::new(Executor))),
    );
    let profile = service
        .save_profile(ProfileInput {
            id: None,
            name: "Primary".into(),
            protocol: ProxyProtocol::Socks5,
            host: "proxy.example.com".into(),
            port: 1080,
            enabled: true,
            authentication_enabled: false,
            credential: None,
        })
        .unwrap();
    // Executor assertions use the actual generated stable ID below.
    let _ = profile;
    let app = mock_builder()
        .manage(service.clone())
        .invoke_handler(tauri::generate_handler![
            crate::ipc_latency::start_proxy_latency_task,
            crate::ipc_latency::get_proxy_latency_task,
            crate::ipc_latency::release_proxy_latency_task,
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
                url: webview.url().unwrap(),
                body: InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
        .map(|body| body.deserialize().unwrap())
    };
    let id = service.list_profiles().unwrap()[0].id.clone();
    assert!(invoke("start_proxy_latency_task", json!({"id":"missing"})).is_err());
    let first = invoke("start_proxy_latency_task", json!({"id":id})).unwrap();
    let second = invoke("start_proxy_latency_task", json!({"id":id})).unwrap();
    assert_eq!(first["task"]["task_id"], second["task"]["task_id"]);
    assert_ne!(first["subscription_id"], second["subscription_id"]);
    assert_eq!(first["task"]["configuration_revision"], 1);
    let first_id = first["subscription_id"].as_str().unwrap();
    let second_id = second["subscription_id"].as_str().unwrap();
    invoke(
        "release_proxy_latency_task",
        json!({"subscriptionId":first_id}),
    )
    .unwrap();
    assert!(invoke("get_proxy_latency_task", json!({"subscriptionId":first_id})).is_err());
    let snapshot = invoke(
        "get_proxy_latency_task",
        json!({"subscriptionId":second_id}),
    )
    .unwrap();
    assert!(matches!(
        snapshot["state"].as_str(),
        Some("queued" | "running")
    ));
    invoke(
        "release_proxy_latency_task",
        json!({"subscriptionId":second_id}),
    )
    .unwrap();
    invoke(
        "release_proxy_latency_task",
        json!({"subscriptionId":second_id}),
    )
    .unwrap();
    assert!(invoke(
        "get_proxy_latency_task",
        json!({"subscriptionId":second_id})
    )
    .is_err());
}
