//! 原生入口到首屏就绪确认的单调耗时；不接受前端提供的时间值。
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

pub(crate) struct StartupMetrics {
    started: Instant,
    acknowledged: AtomicBool,
}

impl StartupMetrics {
    pub(crate) fn new(started: Instant) -> Self {
        Self {
            started,
            acknowledged: AtomicBool::new(false),
        }
    }

    fn acknowledge(&self) -> Option<f64> {
        self.acknowledged
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_previous| self.started.elapsed().as_secs_f64() * 1000.0)
    }
}

/// 首个就绪确认记录一次指标；重载或重复调用不覆盖原生启动样本。
#[tauri::command]
pub(crate) fn acknowledge_frontend_ready(metrics: tauri::State<'_, StartupMetrics>) -> bool {
    let Some(elapsed_ms) = metrics.acknowledge() else {
        return false;
    };
    crate::performance_metrics::record("startup_frontend_ready", elapsed_ms, true);
    tracing::info!(target: "performance", operation = "startup_frontend_ready", elapsed_ms,
        "native frontend ready");
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simultaneous_ready_acknowledgments_record_only_the_first() {
        let metrics = StartupMetrics::new(Instant::now() - std::time::Duration::from_millis(10));
        let results = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..8)
                .map(|_| scope.spawn(|| metrics.acknowledge()))
                .collect();
            workers
                .into_iter()
                .map(|worker| worker.join().expect("worker completes"))
                .collect::<Vec<_>>()
        });
        let elapsed: Vec<_> = results.into_iter().flatten().collect();
        assert_eq!(elapsed.len(), 1);
        assert!(elapsed[0] >= 10.0);
        assert!(metrics.acknowledge().is_none());
    }

    #[test]
    fn readiness_command_is_callable_and_idempotent() -> Result<(), Box<dyn std::error::Error>> {
        use tauri::{
            ipc::{CallbackFn, InvokeBody},
            test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY},
            webview::InvokeRequest,
            WebviewWindowBuilder,
        };
        let app = mock_builder()
            .manage(StartupMetrics::new(Instant::now()))
            .invoke_handler(tauri::generate_handler![acknowledge_frontend_ready])
            .build(mock_context(noop_assets()))?;
        let window = WebviewWindowBuilder::new(&app, "main", Default::default()).build()?;
        for expected in [true, false] {
            let response = get_ipc_response(
                &window,
                InvokeRequest {
                    cmd: "acknowledge_frontend_ready".into(),
                    callback: CallbackFn(0),
                    error: CallbackFn(1),
                    url: window.url()?,
                    body: InvokeBody::Json(serde_json::json!({})),
                    headers: Default::default(),
                    invoke_key: INVOKE_KEY.into(),
                },
            )
            .expect("ready command succeeds")
            .deserialize::<bool>()?;
            assert_eq!(response, expected);
        }
        Ok(())
    }
}
