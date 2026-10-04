//! 显式运行的性能采样；测试宿主初始化不等同于原生桌面启动。
use super::*;
use crate::services::ApplicationService as BenchmarkService;
use serde::Serialize;
use std::{error::Error, time::Instant};

#[derive(Serialize)]
struct Measurement {
    samples: usize,
    median_us: f64,
    p95_us: f64,
    min_us: f64,
    max_us: f64,
}

fn summarize(mut samples: Vec<f64>) -> Measurement {
    samples.sort_by(f64::total_cmp);
    let count = samples.len();
    Measurement {
        samples: count,
        median_us: samples[count / 2],
        p95_us: samples[(count * 95 / 100).min(count - 1)],
        min_us: samples[0],
        max_us: samples[count - 1],
    }
}

fn elapsed_us(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1_000_000.0
}

#[test]
#[ignore = "手动运行以采样性能，不向普通测试施加计时断言"]
fn measure_configuration_and_tauri_dispatch() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let tracing_enabled = std::env::var("ARCHITECTURE_BENCHMARK_TRACING").as_deref() == Ok("1");
    let _worker_guard = if tracing_enabled {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
        let (writer, guard) = crate::trace_writer::asynchronous(&directory.path().join("logs"))?;
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new("info"))
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_ansi(false)
                    .with_writer(writer),
            )
            .try_init()?;
        Some(guard)
    } else {
        None
    };

    let path = directory.path().join("benchmark.sqlite3");
    let store = Arc::new(SqliteConfigurationStore::open(&path)?);
    let mut configuration = PersistedConfiguration::default();
    for index in 0..10 {
        configuration.profiles.push(crate::models::ProxyProfile {
            id: format!("profile-{index}"),
            name: format!("Profile {index}"),
            protocol: crate::models::ProxyProtocol::Socks5,
            host: "proxy.example.com".into(),
            port: 1080,
            authentication_enabled: false,
            credential_ref: None,
            enabled: true,
        });
    }
    store.save(&configuration)?;
    let mut loading = Vec::new();
    // Warm up the database before recording steady-state loads.
    for index in 0..1_100 {
        let started = Instant::now();
        let loaded = store.load()?;
        if index >= 100 {
            loading.push(elapsed_us(started));
        }
        assert_eq!(loaded.profiles.len(), 10);
    }

    let mut bootstrap = Vec::new();
    let mut dispatch = Vec::new();
    let mut profile_save = Vec::new();
    let mut rule_save = Vec::new();
    for index in 0..110 {
        let started = Instant::now();
        let reopened = Arc::new(SqliteConfigurationStore::open(&path)?);
        let runtime = ManagedRuntime::from_lease(
            reopened.load()?,
            Box::new(Arc::new(Backend::default())),
            SessionLease::acquire(&uuid::Uuid::new_v4().to_string())?,
            reopened.load_mode()?,
        )?
        .with_configuration_revision(reopened.recovery_revision()?);
        let service = Arc::new(BenchmarkService::new(
            Box::new(reopened.clone()),
            Box::new(Credentials),
            Box::new(Arc::new(Startup(AtomicBool::new(false)))),
            Box::new(runtime),
        ));
        let app = mock_builder()
            .manage(service.clone())
            .manage(reopened)
            .invoke_handler(tauri::generate_handler![list_profiles])
            .build(mock_context(noop_assets()))?;
        let webview = WebviewWindowBuilder::new(&app, "main", Default::default()).build()?;
        let bootstrap_us = elapsed_us(started);
        let started = Instant::now();
        let response = get_ipc_response(
            &webview,
            InvokeRequest {
                cmd: "list_profiles".into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: webview.url()?,
                body: InvokeBody::Json(json!({})),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
        .map_err(|error| std::io::Error::other(format!("IPC 采样失败：{error}")))?;
        let profiles: Value = response.deserialize()?;
        let dispatch_us = elapsed_us(started);
        assert_eq!(profiles.as_array().map(Vec::len), Some(10));
        let started = Instant::now();
        service.save_profile(crate::services::ProfileInput {
            id: Some("profile-0".into()),
            name: format!("Profile edited {index}"),
            protocol: crate::models::ProxyProtocol::Socks5,
            host: "proxy.example.com".into(),
            port: 1080,
            authentication_enabled: false,
            enabled: true,
            credential: Some(crate::credentials::CredentialUpdate::Preserve),
        })?;
        let profile_save_us = elapsed_us(started);
        let started = Instant::now();
        service.replace_rules(vec![crate::models::RoutingRule {
            id: "benchmark-rule".into(),
            name: format!("Rule edited {index}"),
            matcher: crate::models::RuleMatcher::Domain,
            target: "example.com".into(),
            port_start: None,
            port_end: None,
            action: crate::models::RuleAction::Direct,
            proxy_profile_id: None,
            enabled: true,
        }])?;
        let rule_save_us = elapsed_us(started);
        if index >= 10 {
            profile_save.push(profile_save_us);
            rule_save.push(rule_save_us);
            bootstrap.push(bootstrap_us);
            dispatch.push(dispatch_us);
        }
    }
    let report = json!({
        "platform": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "build": if cfg!(debug_assertions) { "debug" } else { "release" },
        "json_tracing_enabled": tracing_enabled,
        "revision": std::env::var("ARCHITECTURE_BENCHMARK_REVISION").unwrap_or_default(),
        "profile_count": 10,
        "configuration_load": summarize(loading),
        "mock_application_bootstrap": summarize(bootstrap),
        "tauri_command_dispatch": summarize(dispatch),
        "successful_profile_save": summarize(profile_save),
        "successful_rule_save": summarize(rule_save),
        "limitations": ["测试宿主初始化不包含原生窗口、托盘或真实内核启动", "IPC 采样不包含 WebView JavaScript 往返", "未测量 Windows 系统代理接管"]
    });
    let output = std::env::var("ARCHITECTURE_BENCHMARK_OUTPUT")?;
    std::fs::write(output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
