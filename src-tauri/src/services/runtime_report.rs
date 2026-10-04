//! 只读诊断工具：导出运行时与指标，不读取或序列化认证信息。
use super::ApplicationService;
use crate::{error::AppError, error_context::ErrorDomain};
use serde::Serialize;

impl ApplicationService {
    /// 导出当前运行时、转换历史和有界性能指标的 JSON 快照。
    pub fn export_runtime_snapshot(&self) -> Result<String, AppError> {
        self.finish(
            (|| {
                let snapshot = self.runtime_snapshot();
                let report = serde_json::json!({
                    "schema_version": 1,
                    "created_at_ms": crate::store::diagnostic_now_ms()?,
                    "runtime": snapshot,
                    "transitions": self.context.runtime.transition_history(),
                    "metrics": crate::performance_metrics::snapshot(),
                    "trace_dropped_records": crate::tracing_setup::dropped_records(),
                });
                serde_json::to_string_pretty(&report)
                    .map_err(|_| AppError::unavailable("运行时快照无法导出"))
            })(),
            ErrorDomain::Application,
            "export_runtime_snapshot",
        )
    }

    /// 校验当前持久配置并返回恢复阻塞与运行时健康状态，不改变配置。
    pub fn validate_configuration(&self) -> Result<String, AppError> {
        self.finish(
            (|| {
                let health = self.health_check()?;
                #[derive(Serialize)]
                struct Report {
                    valid: bool,
                    configuration_revision: u64,
                    recovery_pending: bool,
                    runtime: crate::runtime::RuntimeSnapshot,
                }
                serde_json::to_string(&Report {
                    valid: true,
                    configuration_revision: health.configuration_revision,
                    recovery_pending: health.recovery_pending,
                    runtime: health.runtime,
                })
                .map_err(|_| AppError::unavailable("配置校验报告无法生成"))
            })(),
            ErrorDomain::Application,
            "validate_configuration",
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::services::application_service::tests::Fixture;
    #[test]
    fn reports_are_parseable_and_exclude_credentials_and_configuration_inputs(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = Fixture::new();
        let report: serde_json::Value =
            serde_json::from_str(&fixture.service.export_runtime_snapshot()?)?;
        assert_eq!(report["schema_version"], 1);
        assert!(report["runtime"].is_object());
        assert!(report["metrics"].is_array());
        assert!(report["transitions"].is_array());
        let validation: serde_json::Value =
            serde_json::from_str(&fixture.service.validate_configuration()?)?;
        assert_eq!(validation["valid"], true);
        assert_eq!(validation["recovery_pending"], false);
        for value in [report, validation] {
            let output = serde_json::to_string(&value)?;
            for private in ["credential", "password", "username", "profiles", "host"] {
                assert!(!output.contains(private));
            }
        }
        Ok(())
    }
}
