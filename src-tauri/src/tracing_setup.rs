//! 生产 JSON Lines 与开发终端格式共享脱敏追踪事件。
use crate::trace_writer;
#[cfg(test)]
use crate::trace_writer::TraceWriter;
use std::path::Path;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// 日志故障不阻止配置访问或网络恢复；只输出固定错误说明。
#[derive(Default)]
#[must_use]
pub(crate) struct ProfilingGuard {
    flame: std::sync::Mutex<Option<tracing_flame::FlushGuard<std::io::BufWriter<std::fs::File>>>>,
    worker: Option<tracing_appender::non_blocking::WorkerGuard>,
}

impl Drop for ProfilingGuard {
    fn drop(&mut self) {
        if let Ok(guard) = self.flame.get_mut() {
            guard.take();
        }
        self.worker.take();
    }
}

static DROPPED: std::sync::OnceLock<tracing_appender::non_blocking::ErrorCounter> =
    std::sync::OnceLock::new();
pub(crate) fn dropped_records() -> usize {
    DROPPED.get().map_or(0, |counter| counter.dropped_lines())
}

pub(crate) fn initialize(directory: &Path) -> ProfilingGuard {
    let (writer, worker) = match trace_writer::asynchronous(directory) {
        Ok(writer) => writer,
        Err(_) => {
            eprintln!("无法初始化运行时追踪文件");
            return ProfilingGuard::default();
        }
    };
    let _ = DROPPED.set(writer.error_counter());
    let (flame, guard) = if std::env::var("SOCKS_PROXY_PROFILE").as_deref() == Ok("1") {
        match tracing_flame::FlameLayer::with_file(directory.join("profile.folded")) {
            Ok((layer, guard)) => (
                Some(
                    layer
                        .with_file_and_line(false)
                        .with_empty_samples(false)
                        .with_threads_collapsed(true),
                ),
                Some(guard),
            ),
            Err(_) => {
                eprintln!("无法初始化性能分析文件");
                (None, None)
            }
        }
    } else {
        (None, None)
    };
    let filter = filter(std::env::var("RUST_LOG").ok().as_deref());
    let registry = tracing_subscriber::registry().with(filter).with(
        fmt::layer()
            .json()
            .with_ansi(false)
            .with_target(true)
            .with_writer(writer)
            .with_filter(tracing_subscriber::filter::filter_fn(allowed_target)),
    );
    let registry = registry.with(flame);
    let result = if cfg!(debug_assertions) {
        registry
            .with(
                fmt::layer()
                    .pretty()
                    .with_writer(std::io::stderr)
                    .with_filter(tracing_subscriber::filter::filter_fn(allowed_target)),
            )
            .try_init()
    } else {
        registry.try_init()
    };
    if result.is_err() {
        eprintln!("运行时追踪订阅器未安装");
    }
    ProfilingGuard {
        flame: std::sync::Mutex::new(guard),
        worker: Some(worker),
    }
}
fn allowed_target(metadata: &tracing::Metadata<'_>) -> bool {
    metadata.target().starts_with("socks_proxy_lib::")
        || matches!(
            metadata.target(),
            "runtime_transition" | "application_error" | "performance"
        )
}
fn filter(value: Option<&str>) -> EnvFilter {
    value
        .and_then(|value| EnvFilter::try_new(value).ok())
        .unwrap_or_else(|| EnvFilter::new("info"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filter_accepts_valid_configuration_and_falls_back_for_invalid_values() {
        assert_eq!(filter(None).to_string(), "info");
        assert_eq!(
            filter(Some("debug,runtime_transition=trace")).to_string(),
            "runtime_transition=trace,debug"
        );
        assert_eq!(filter(Some("invalid[syntax")).to_string(), "info");
    }
    #[test]
    fn file_subscriber_produces_parseable_json_with_span_context(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let writer = TraceWriter::open(directory.path())?;
        let subscriber = fmt().json().with_ansi(false).with_writer(writer).finish();
        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("runtime_transition", operation_id = 42);
            let _entered = span.enter();
            tracing::info!(event = "health_check_passed", "runtime state changed");
        });
        let output = std::fs::read_to_string(directory.path().join("runtime.jsonl"))?;
        let event: serde_json::Value = serde_json::from_str(output.trim())?;
        assert_eq!(event["fields"]["event"], "health_check_passed");
        assert_eq!(event["span"]["operation_id"], 42);
        Ok(())
    }
    #[test]
    fn dependency_events_are_excluded_even_when_debugging_is_enabled(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let subscriber = tracing_subscriber::registry().with(
            fmt::layer()
                .json()
                .with_ansi(false)
                .with_writer(TraceWriter::open(directory.path())?)
                .with_filter(tracing_subscriber::filter::filter_fn(allowed_target)),
        );
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(target: "reqwest", url = "private-address", "private-dependency-value");
            tracing::info!(target: "application_error", error_id = "safe-id", "application operation failed");
        });
        let output = std::fs::read_to_string(directory.path().join("runtime.jsonl"))?;
        assert_eq!(output.lines().count(), 1);
        assert!(output.contains("safe-id"));
        assert!(!output.contains("private"));
        Ok(())
    }

    #[test]
    fn failed_file_initialization_does_not_install_or_replace_a_subscriber() -> std::io::Result<()>
    {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("occupied");
        std::fs::write(&path, "existing")?;
        let _guard = initialize(&path);
        assert_eq!(std::fs::read_to_string(&path)?, "existing");
        Ok(())
    }
    #[test]
    fn profiling_emits_folded_stacks_without_span_inputs() -> Result<(), Box<dyn std::error::Error>>
    {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("profile.folded");
        let (layer, guard) = tracing_flame::FlameLayer::with_file(&path)?;
        let subscriber = tracing_subscriber::registry()
            .with(layer.with_file_and_line(false).with_empty_samples(false));
        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("profile_operation", password = "private-secret");
            let _entered = span.enter();
            std::thread::sleep(std::time::Duration::from_millis(1));
        });
        guard.flush()?;
        let output = std::fs::read_to_string(path)?;
        assert!(output.contains("profile_operation"));
        assert!(!output.contains("private-secret"));
        assert!(!output.contains("/Users/"));
        Ok(())
    }
    #[test]
    #[ignore = "isolated tracing subscriber child invoked by parent test"]
    fn subscriber_child() -> Result<(), Box<dyn std::error::Error>> {
        let directory =
            std::path::PathBuf::from(std::env::var("ARCHITECTURE_TRACE_TEST_DIRECTORY")?);
        let guard = initialize(&directory);
        let span = tracing::info_span!("subscriber_validation");
        let entered = span.enter();
        tracing::info!(target: "runtime_transition", event = "validated", "runtime state changed");
        tracing::debug!(target: "runtime_transition", event = "hidden", "debug state changed");
        let _duplicate = initialize(&directory);
        drop(entered);
        drop(guard);
        Ok(())
    }
    #[test]
    fn initialization_installs_filtered_json_subscriber_and_flushes_profiling(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let output = std::process::Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "tracing_setup::tests::subscriber_child",
                "--ignored",
            ])
            .env("ARCHITECTURE_TRACE_TEST_DIRECTORY", directory.path())
            .env("RUST_LOG", "info")
            .env("SOCKS_PROXY_PROFILE", "1")
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let content = std::fs::read_to_string(directory.path().join("runtime.jsonl"))?;
        assert!(content.contains("validated"));
        assert!(!content.contains("hidden"));
        for line in content.lines() {
            let _: serde_json::Value = serde_json::from_str(line)?;
        }
        assert!(directory.path().join("profile.folded").exists());
        Ok(())
    }
}
