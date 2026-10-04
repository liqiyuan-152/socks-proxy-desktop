//! 持久化诊断的快照导出与错误聚合；原始记录保留各次失败身份。
use super::{
    diagnostics::{DiagnosticFilter, RuntimeDiagnostic, FILTER_SQL},
    storage_error, AppError, SqliteConfigurationStore,
};
use rusqlite::{params_from_iter, Row};
use serde::Serialize;

/// 同一领域错误在同一操作与严重级别下的发生情况。
#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct DiagnosticGroup {
    pub error_type: String,
    pub operation: Option<String>,
    pub severity: String,
    pub occurrences: u64,
    pub first_seen_ms: i64,
    pub last_seen_ms: i64,
}

fn diagnostic(row: &Row<'_>) -> rusqlite::Result<RuntimeDiagnostic> {
    Ok(RuntimeDiagnostic {
        id: row.get(0)?,
        created_at_ms: row.get(1)?,
        severity: row.get(2)?,
        summary: row.get(3)?,
        error_type: row.get(4)?,
        operation: row.get(5)?,
    })
}

impl SqliteConfigurationStore {
    /// 按稳定类型、操作、严重级别聚合；过滤与列表具有相同语义。
    pub fn diagnostic_groups(
        &self,
        filter: &DiagnosticFilter,
    ) -> Result<Vec<DiagnosticGroup>, AppError> {
        filter.validate()?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::storage("诊断存储锁不可用"))?;
        let sql = format!(
            "SELECT error_type, operation, severity, COUNT(*), MIN(created_at_ms), MAX(created_at_ms)
             FROM runtime_diagnostics {FILTER_SQL} AND error_type IS NOT NULL
             GROUP BY error_type, operation, severity
             ORDER BY MAX(created_at_ms) DESC, error_type, operation, severity"
        );
        let mut statement = connection.prepare(&sql).map_err(storage_error)?;
        let groups = statement
            .query_map(params_from_iter(filter.sql_params()), |row| {
                Ok(DiagnosticGroup {
                    error_type: row.get(0)?,
                    operation: row.get(1)?,
                    severity: row.get(2)?,
                    occurrences: row.get(3)?,
                    first_seen_ms: row.get(4)?,
                    last_seen_ms: row.get(5)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(groups)
    }

    /// 导出匹配的完整快照，每行一个 JSON 对象，不受界面分页限制。
    pub fn export_diagnostics(&self, filter: &DiagnosticFilter) -> Result<String, AppError> {
        filter.validate()?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::storage("诊断存储锁不可用"))?;
        let sql = format!(
            "SELECT id, created_at_ms, severity, summary, error_type, operation
             FROM runtime_diagnostics {FILTER_SQL} ORDER BY created_at_ms DESC, rowid DESC"
        );
        let mut statement = connection.prepare(&sql).map_err(storage_error)?;
        let rows = statement
            .query_map(params_from_iter(filter.sql_params()), diagnostic)
            .map_err(storage_error)?;
        let mut output = String::new();
        for row in rows {
            let item = row.map_err(storage_error)?;
            output.push_str(
                &serde_json::to_string(&item).map_err(|_| AppError::storage("诊断序列化失败"))?,
            );
            output.push('\n');
        }
        Ok(output)
    }
}

#[cfg(test)]
#[path = "store_diagnostic_report_tests.rs"]
mod tests;
