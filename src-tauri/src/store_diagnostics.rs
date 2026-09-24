use super::*;
use rusqlite::{params, params_from_iter, types::Value as SqlValue};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

const PERMANENT_DIAGNOSTIC_LIMIT: i64 = 100_000;
const PAGE_LIMIT: usize = 100;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeDiagnostic {
    pub id: String,
    pub created_at_ms: i64,
    pub severity: String,
    pub summary: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct DiagnosticFilter {
    pub from_ms: Option<i64>,
    pub until_ms: Option<i64>,
    pub severity: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DiagnosticPage {
    pub items: Vec<RuntimeDiagnostic>,
    pub total: u64,
    pub next_offset: Option<usize>,
}

impl DiagnosticFilter {
    fn validate(&self) -> Result<(), AppError> {
        if self
            .from_ms
            .zip(self.until_ms)
            .is_some_and(|(from, until)| from >= until)
        {
            return Err(AppError::unavailable("诊断查询时间范围无效"));
        }
        if self
            .severity
            .as_deref()
            .is_some_and(|level| !matches!(level, "info" | "warning" | "error"))
        {
            return Err(AppError::unavailable("诊断级别无效"));
        }
        Ok(())
    }

    fn sql_params(&self) -> Vec<SqlValue> {
        vec![
            self.from_ms.map_or(SqlValue::Null, SqlValue::Integer),
            self.until_ms.map_or(SqlValue::Null, SqlValue::Integer),
            self.severity
                .as_ref()
                .map_or(SqlValue::Null, |s| SqlValue::Text(s.clone())),
        ]
    }
}

const FILTER_SQL: &str = "WHERE (?1 IS NULL OR created_at_ms >= ?1)
    AND (?2 IS NULL OR created_at_ms < ?2)
    AND (?3 IS NULL OR severity = ?3)";

impl SqliteConfigurationStore {
    pub fn record_diagnostic(&self, diagnostic: &RuntimeDiagnostic) -> Result<(), AppError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| AppError::storage("配置存储锁不可用"))?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let policy = read_configuration(&transaction)?
            .settings
            .diagnostic_retention;
        transaction.execute(
            "INSERT INTO runtime_diagnostics (id, created_at_ms, severity, summary) VALUES (?1, ?2, ?3, ?4)",
            params![diagnostic.id, diagnostic.created_at_ms, diagnostic.severity, diagnostic.summary],
        ).map_err(storage_error)?;
        prune_diagnostics(&transaction, policy, now_ms()?)?;
        transaction.commit().map_err(storage_error)?;
        Ok(())
    }

    pub fn list_diagnostics(
        &self,
        filter: &DiagnosticFilter,
        offset: usize,
        limit: usize,
    ) -> Result<DiagnosticPage, AppError> {
        filter.validate()?;
        if limit == 0 || limit > PAGE_LIMIT || offset > 100_000 {
            return Err(AppError::unavailable("诊断分页参数超出范围"));
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::storage("诊断存储锁不可用"))?;
        let params = filter.sql_params();
        let count_sql = format!("SELECT COUNT(*) FROM runtime_diagnostics {FILTER_SQL}");
        let total: u64 = connection
            .query_row(&count_sql, params_from_iter(&params), |row| row.get(0))
            .map_err(storage_error)?;
        let page_sql = format!(
            "SELECT id, created_at_ms, severity, summary FROM runtime_diagnostics {FILTER_SQL}
            ORDER BY created_at_ms DESC, rowid DESC LIMIT ?4 OFFSET ?5"
        );
        let mut page_params = params;
        page_params.push(SqlValue::Integer(limit as i64));
        page_params.push(SqlValue::Integer(offset as i64));
        let mut statement = connection.prepare(&page_sql).map_err(storage_error)?;
        let items = statement
            .query_map(params_from_iter(&page_params), |row| {
                Ok(RuntimeDiagnostic {
                    id: row.get(0)?,
                    created_at_ms: row.get(1)?,
                    severity: row.get(2)?,
                    summary: row.get(3)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        let next_offset = offset
            .checked_add(items.len())
            .filter(|next| (*next as u64) < total);
        Ok(DiagnosticPage {
            items,
            total,
            next_offset,
        })
    }

    pub fn clear_diagnostics(
        &self,
        filter: &DiagnosticFilter,
        confirmed: bool,
    ) -> Result<usize, AppError> {
        if !confirmed {
            return Err(AppError::unavailable("清理诊断需要用户确认"));
        }
        filter.validate()?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::storage("诊断存储锁不可用"))?;
        let sql = format!("DELETE FROM runtime_diagnostics {FILTER_SQL}");
        connection
            .execute(&sql, params_from_iter(filter.sql_params()))
            .map_err(storage_error)
    }

    #[cfg(test)]
    pub fn apply_diagnostic_retention(
        &self,
        policy: RetentionPolicy,
        at_ms: i64,
    ) -> Result<usize, AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::storage("配置存储锁不可用"))?;
        prune_diagnostics(&connection, policy, at_ms)
    }

    #[cfg(test)]
    pub fn diagnostic_count(&self) -> Result<u64, AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::storage("配置存储锁不可用"))?;
        connection
            .query_row("SELECT COUNT(*) FROM runtime_diagnostics", [], |row| {
                row.get(0)
            })
            .map_err(storage_error)
    }
}

pub(super) fn prune_diagnostics(
    connection: &Connection,
    policy: RetentionPolicy,
    at_ms: i64,
) -> Result<usize, AppError> {
    match policy {
        RetentionPolicy::Permanent => connection
            .execute(
                "DELETE FROM runtime_diagnostics WHERE id NOT IN (
                SELECT id FROM runtime_diagnostics ORDER BY created_at_ms DESC, rowid DESC LIMIT ?1
            )",
                [PERMANENT_DIAGNOSTIC_LIMIT],
            )
            .map_err(storage_error),
        policy => {
            let days: i64 = match policy {
                RetentionPolicy::Days7 => 7,
                RetentionPolicy::Days30 => 30,
                RetentionPolicy::Days90 => 90,
                RetentionPolicy::Permanent => unreachable!(),
            };
            let cutoff = at_ms.saturating_sub(days * 24 * 60 * 60 * 1000);
            connection
                .execute(
                    "DELETE FROM runtime_diagnostics WHERE created_at_ms < ?1",
                    [cutoff],
                )
                .map_err(storage_error)
        }
    }
}

pub(super) fn now_ms() -> Result<i64, AppError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::storage("系统时钟不可用"))?;
    i64::try_from(duration.as_millis()).map_err(|_| AppError::storage("系统时钟超出范围"))
}

#[cfg(test)]
#[path = "store_diagnostics_tests.rs"]
mod tests;
