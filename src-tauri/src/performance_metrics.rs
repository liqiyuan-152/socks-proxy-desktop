//! 有界进程内指标；仅保存固定操作名、耗时与结果，不包含业务参数。
use serde::Serialize;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Mutex, OnceLock},
    time::Instant,
};
const SAMPLE_LIMIT: usize = 128;

/// 累计计数与最近 128 次操作的耗时分位数。
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct PerformanceMetric {
    pub operation: String,
    pub count: u64,
    pub failures: u64,
    pub total_ms: f64,
    pub maximum_ms: f64,
    pub recent_p50_ms: f64,
    pub recent_p95_ms: f64,
    pub retained_samples: usize,
}
#[derive(Default)]
struct Samples {
    count: u64,
    failures: u64,
    total_ms: f64,
    maximum_ms: f64,
    recent: VecDeque<f64>,
}
#[derive(Default)]
struct Metrics(BTreeMap<&'static str, Samples>);
impl Metrics {
    fn record(&mut self, operation: &'static str, elapsed_ms: f64, succeeded: bool) {
        let samples = self.0.entry(operation).or_default();
        samples.count += 1;
        samples.failures += u64::from(!succeeded);
        samples.total_ms += elapsed_ms;
        samples.maximum_ms = samples.maximum_ms.max(elapsed_ms);
        if samples.recent.len() == SAMPLE_LIMIT {
            samples.recent.pop_front();
        }
        samples.recent.push_back(elapsed_ms);
    }
    fn snapshot(&self) -> Vec<PerformanceMetric> {
        self.0
            .iter()
            .map(|(operation, samples)| {
                let mut sorted: Vec<_> = samples.recent.iter().copied().collect();
                sorted.sort_by(f64::total_cmp);
                let percentile = |percent: usize| {
                    let index = (sorted.len() * percent).div_ceil(100).saturating_sub(1);
                    sorted.get(index).copied().unwrap_or_default()
                };
                PerformanceMetric {
                    operation: (*operation).into(),
                    count: samples.count,
                    failures: samples.failures,
                    total_ms: samples.total_ms,
                    maximum_ms: samples.maximum_ms,
                    recent_p50_ms: percentile(50),
                    recent_p95_ms: percentile(95),
                    retained_samples: sorted.len(),
                }
            })
            .collect()
    }
}
static METRICS: OnceLock<Mutex<Metrics>> = OnceLock::new();

pub(crate) fn measure<T, E>(
    operation: &'static str,
    action: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let start = Instant::now();
    let result = action();
    record(
        operation,
        start.elapsed().as_secs_f64() * 1000.0,
        result.is_ok(),
    );
    result
}
pub(crate) fn record(operation: &'static str, elapsed_ms: f64, succeeded: bool) {
    if let Ok(mut metrics) = METRICS.get_or_init(Mutex::default).lock() {
        metrics.record(operation, elapsed_ms, succeeded);
    }
    tracing::debug!(target: "performance", operation, elapsed_ms, succeeded, "operation measured");
}
/// 获取一致的有界指标快照，不触发后端或配置读取。
pub fn snapshot() -> Vec<PerformanceMetric> {
    METRICS
        .get_or_init(Mutex::default)
        .lock()
        .map(|metrics| metrics.snapshot())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metrics_keep_exact_counters_and_bound_recent_percentiles() {
        let mut metrics = Metrics::default();
        for index in 1..=200 {
            metrics.record("test", f64::from(index), index % 2 == 0);
        }
        let report = metrics.snapshot();
        assert_eq!(report[0].count, 200);
        assert_eq!(report[0].failures, 100);
        assert_eq!(report[0].total_ms, 20100.0);
        assert_eq!(report[0].maximum_ms, 200.0);
        assert_eq!(report[0].retained_samples, 128);
        assert_eq!(report[0].recent_p50_ms, 136.0);
        assert_eq!(report[0].recent_p95_ms, 194.0);
    }
    #[test]
    fn measure_preserves_result_and_monotonic_elapsed_time() {
        let start = Instant::now();
        let result = measure("test_measure_success", || {
            std::thread::sleep(std::time::Duration::from_millis(2));
            Ok::<_, ()>(42)
        });
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(result, Ok(42));
        assert_eq!(
            measure("test_measure_failure", || Err::<(), _>("unchanged")),
            Err("unchanged")
        );
        let reports = snapshot();
        let sample = reports
            .iter()
            .find(|report| report.operation == "test_measure_success")
            .expect("sample recorded");
        assert!(sample.total_ms >= 2.0 && sample.total_ms <= elapsed);
        let failure = reports
            .iter()
            .find(|report| report.operation == "test_measure_failure")
            .expect("failure recorded");
        assert_eq!(failure.failures, 1);
    }
}
