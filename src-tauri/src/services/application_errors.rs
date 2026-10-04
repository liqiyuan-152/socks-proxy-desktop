//! 应用边界统一补充、记录错误；记录失败不替代原来的业务失败。
use super::application_service::ApplicationService;
use crate::{
    error::AppError,
    error_context::{ErrorContext, ErrorDomain},
    store::RuntimeDiagnostic,
};
use serde::Serialize;

#[derive(Serialize)]
struct ErrorDiagnostic<'a> {
    summary: &'a str,
    context: &'a ErrorContext,
    stack: &'a [String],
}

pub(super) fn annotate(
    mut error: AppError,
    domain: ErrorDomain,
    operation: &'static str,
) -> AppError {
    let kind = match error.code.as_str() {
        "validation_error" => "validation_failed",
        "storage_error" => "storage_failed",
        "credential_error" => "credential_invalid",
        "not_found" => "not_found",
        "configuration_recovery" => "recovery_in_progress",
        "rollback_failed" => "rollback_failed",
        "unavailable" => "unavailable",
        _ => "operation_failed",
    };
    let domain = match error.code.as_str() {
        "storage_error" => ErrorDomain::Storage,
        "credential_error" => ErrorDomain::Credential,
        _ => domain,
    };
    error = error.with_context(domain, kind);
    if let Some(context) = error.context.as_mut() {
        if context.operation.is_none() {
            context.operation = Some(operation.into());
        }
        let span = tracing::info_span!(target: "application_error", "application_failure",
            trace_id = context.trace_id.as_deref().unwrap_or(""), error_id = %context.error_id);
        if context.span_id.is_none() {
            context.span_id = span.id().map(|id| format!("{:016x}", id.into_u64()));
        }
        let _entered = span.enter();
        // Do not log user-supplied messages, fields, credentials or input values.
        tracing::error!(target: "application_error", error_id = %context.error_id,
            domain = ?context.domain, kind = %context.kind, operation,
            trace_id = context.trace_id.as_deref().unwrap_or(""),
            span_id = context.span_id.as_deref().unwrap_or(""), "application operation failed");
    }
    error
}

impl ApplicationService {
    pub(super) fn finish<T, E: Into<AppError>>(
        &self,
        result: Result<T, E>,
        domain: ErrorDomain,
        operation: &'static str,
    ) -> Result<T, AppError> {
        result.map_err(|error| {
            let error = annotate(error.into(), domain, operation);
            self.record_error(&error, "应用操作失败");
            error
        })
    }

    pub(super) fn record_error(&self, error: &AppError, summary: &str) {
        let Some(context) = error.context.as_ref() else {
            return;
        };
        let diagnostic = ErrorDiagnostic {
            summary,
            context,
            stack: &context.stack,
        };
        let Ok(summary) = serde_json::to_string(&diagnostic) else {
            return;
        };
        let recorded = self.context.store.record_diagnostic(&RuntimeDiagnostic {
            error_type: Some(format!(
                "{}.{}",
                serde_json::to_value(context.domain)
                    .unwrap_or_default()
                    .as_str()
                    .unwrap_or("application"),
                context.kind
            )),
            operation: context.operation.clone(),
            id: context.error_id.clone(),
            created_at_ms: context.timestamp_ms.min(i64::MAX as u64) as i64,
            severity: if matches!(
                error.code.as_str(),
                "validation_error" | "not_found" | "proxy_not_found" | "proxy_in_use"
            ) {
                "warning"
            } else {
                "error"
            }
            .into(),
            summary,
        });
        if recorded.is_err() {
            tracing::warn!(target: "application_error", error_id = %context.error_id, "diagnostic persistence failed");
        }
    }
}

#[cfg(test)]
#[path = "application_error_tests.rs"]
mod tests;
