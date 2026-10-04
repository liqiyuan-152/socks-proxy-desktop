use crate::{
    error::AppError,
    models::{PersistedConfiguration, RetentionPolicy, RuntimeMode},
};
use rusqlite::{Connection, OptionalExtension};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

const DATABASE_SCHEMA_VERSION: i64 = 6;
#[path = "store_connection_samples.rs"]
mod connection_samples;
use diagnostics::{now_ms, prune_diagnostics};
#[path = "store_diagnostic_report.rs"]
mod diagnostic_report;
pub use diagnostic_report::DiagnosticGroup;
pub use diagnostics::{DiagnosticFilter, DiagnosticPage, RuntimeDiagnostic};

pub fn diagnostic_now_ms() -> Result<i64, AppError> {
    now_ms()
}

pub trait ConfigurationStore: Send + Sync {
    /// 每分钟至多保存一个有效活跃连接数，并返回本地昨日采样。
    fn connection_trend(
        &self,
        _: usize,
        _: i64,
    ) -> Result<Option<crate::observability::ConnectionTrend>, AppError> {
        Err(AppError::unavailable("此配置存储不支持连接数历史采样"))
    }
    /// 查询持久化运行时诊断；不支持诊断的适配器明确返回能力错误。
    fn list_diagnostics(
        &self,
        _: &DiagnosticFilter,
        _: usize,
        _: usize,
    ) -> Result<DiagnosticPage, AppError> {
        Err(AppError::unavailable("此配置存储不支持运行时诊断"))
    }
    /// 记录运行时诊断，存储适配器负责保留策略和容量上限。
    fn record_diagnostic(&self, _: &RuntimeDiagnostic) -> Result<(), AppError> {
        Err(AppError::unavailable("此配置存储不支持运行时诊断"))
    }
    /// 仅在用户确认后清理匹配的诊断。
    fn clear_diagnostics(&self, _: &DiagnosticFilter, _: bool) -> Result<usize, AppError> {
        Err(AppError::unavailable("此配置存储不支持运行时诊断"))
    }
    /// 聚合同类错误，保留底层的单次错误记录。
    fn diagnostic_groups(&self, _: &DiagnosticFilter) -> Result<Vec<DiagnosticGroup>, AppError> {
        Err(AppError::unavailable("此配置存储不支持诊断聚合"))
    }
    /// 导出完整过滤快照为 JSON Lines。
    fn export_diagnostics(&self, _: &DiagnosticFilter) -> Result<String, AppError> {
        Err(AppError::unavailable("此配置存储不支持诊断导出"))
    }
    fn load(&self) -> Result<PersistedConfiguration, AppError>;
    #[cfg(test)]
    fn save(&self, configuration: &PersistedConfiguration) -> Result<(), AppError>;
    fn load_mode(&self) -> Result<RuntimeMode, AppError>;
    fn save_mode(&self, mode: RuntimeMode) -> Result<(), AppError>;
    fn recovery_revision(&self) -> Result<u64, AppError> {
        Err(AppError::unavailable("此配置存储不支持持久化恢复"))
    }
    fn recovery_record(
        &self,
    ) -> Result<Option<crate::configuration_recovery::RecoveryRecord>, AppError> {
        Err(AppError::unavailable("此配置存储不支持持久化恢复"))
    }
    fn begin_recovery(
        &self,
        _: &crate::configuration_recovery::RecoveryIntent,
    ) -> Result<(), AppError> {
        Err(AppError::unavailable("此配置存储不支持持久化恢复"))
    }
    fn commit_recovery(&self, _: &str) -> Result<(), AppError> {
        Err(AppError::unavailable("此配置存储不支持持久化恢复"))
    }
    fn clear_recovery(&self, _: &str) -> Result<(), AppError> {
        Err(AppError::unavailable("此配置存储不支持持久化恢复"))
    }
}

pub struct SqliteConfigurationStore {
    connection: Mutex<Connection>,
}

impl SqliteConfigurationStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let connection = Connection::open(path).map_err(storage_error)?;
        Self::from_connection(connection)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, AppError> {
        let connection = Connection::open_in_memory().map_err(storage_error)?;
        Self::from_connection(connection)
    }

    fn from_connection(connection: Connection) -> Result<Self, AppError> {
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(storage_error)?;
        migrate(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}

impl ConfigurationStore for SqliteConfigurationStore {
    fn connection_trend(
        &self,
        count: usize,
        now: i64,
    ) -> Result<Option<crate::observability::ConnectionTrend>, AppError> {
        connection_samples::record(self, count, now)
    }
    fn diagnostic_groups(
        &self,
        filter: &DiagnosticFilter,
    ) -> Result<Vec<DiagnosticGroup>, AppError> {
        SqliteConfigurationStore::diagnostic_groups(self, filter)
    }
    fn export_diagnostics(&self, filter: &DiagnosticFilter) -> Result<String, AppError> {
        SqliteConfigurationStore::export_diagnostics(self, filter)
    }

    fn list_diagnostics(
        &self,
        filter: &DiagnosticFilter,
        offset: usize,
        limit: usize,
    ) -> Result<DiagnosticPage, AppError> {
        SqliteConfigurationStore::list_diagnostics(self, filter, offset, limit)
    }
    fn record_diagnostic(&self, diagnostic: &RuntimeDiagnostic) -> Result<(), AppError> {
        SqliteConfigurationStore::record_diagnostic(self, diagnostic)
    }
    fn clear_diagnostics(
        &self,
        filter: &DiagnosticFilter,
        confirmed: bool,
    ) -> Result<usize, AppError> {
        SqliteConfigurationStore::clear_diagnostics(self, filter, confirmed)
    }
    fn recovery_revision(&self) -> Result<u64, AppError> {
        SqliteConfigurationStore::recovery_revision(self)
    }
    fn recovery_record(
        &self,
    ) -> Result<Option<crate::configuration_recovery::RecoveryRecord>, AppError> {
        SqliteConfigurationStore::recovery_record(self)
    }
    fn begin_recovery(
        &self,
        intent: &crate::configuration_recovery::RecoveryIntent,
    ) -> Result<(), AppError> {
        SqliteConfigurationStore::begin_recovery(self, intent)
    }
    fn commit_recovery(&self, id: &str) -> Result<(), AppError> {
        SqliteConfigurationStore::commit_recovery(self, id)
    }
    fn clear_recovery(&self, id: &str) -> Result<(), AppError> {
        SqliteConfigurationStore::clear_recovery(self, id)
    }
    fn load(&self) -> Result<PersistedConfiguration, AppError> {
        crate::performance_metrics::measure("configuration_load", || {
            let connection = self
                .connection
                .lock()
                .map_err(|_| AppError::storage("配置存储锁不可用"))?;
            read_configuration(&connection)
        })
    }

    #[cfg(test)]
    fn save(&self, configuration: &PersistedConfiguration) -> Result<(), AppError> {
        configuration.validate()?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| AppError::storage("配置存储锁不可用"))?;
        let transaction = connection.transaction().map_err(storage_error)?;
        if recovery::read_record(&transaction)?.is_some() {
            return Err(crate::configuration_recovery::recovery_error());
        }
        write_configuration(&transaction, configuration)?;
        transaction
            .execute(
                "UPDATE configuration_commit SET revision = revision + 1 WHERE id = 1",
                [],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        Ok(())
    }

    fn load_mode(&self) -> Result<RuntimeMode, AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::storage("配置存储锁不可用"))?;
        let mode: Option<String> = connection
            .query_row("SELECT mode FROM selected_mode WHERE id = 1", [], |row| {
                row.get(0)
            })
            .optional()
            .map_err(storage_error)?;
        mode.map(|value| match value.as_str() {
            "rules" => Ok(RuntimeMode::Rules),
            "global" => Ok(RuntimeMode::Global),
            "direct" => Ok(RuntimeMode::Direct),
            _ => Err(AppError::storage("已保存的代理模式无效")),
        })
        .unwrap_or(Ok(RuntimeMode::Direct))
    }

    fn save_mode(&self, mode: RuntimeMode) -> Result<(), AppError> {
        let value = match mode {
            RuntimeMode::Rules => "rules",
            RuntimeMode::Global => "global",
            RuntimeMode::Direct => "direct",
        };
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::storage("配置存储锁不可用"))?;
        if recovery::read_record(&connection)?.is_some() {
            return Err(crate::configuration_recovery::recovery_error());
        }
        connection
            .execute(
                "INSERT INTO selected_mode (id, mode) VALUES (1, ?1)
                 ON CONFLICT(id) DO UPDATE SET mode = excluded.mode",
                [value],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

impl ConfigurationStore for Arc<SqliteConfigurationStore> {
    fn connection_trend(
        &self,
        count: usize,
        now: i64,
    ) -> Result<Option<crate::observability::ConnectionTrend>, AppError> {
        self.as_ref().connection_trend(count, now)
    }
    fn diagnostic_groups(
        &self,
        filter: &DiagnosticFilter,
    ) -> Result<Vec<DiagnosticGroup>, AppError> {
        self.as_ref().diagnostic_groups(filter)
    }
    fn export_diagnostics(&self, filter: &DiagnosticFilter) -> Result<String, AppError> {
        self.as_ref().export_diagnostics(filter)
    }

    fn list_diagnostics(
        &self,
        filter: &DiagnosticFilter,
        offset: usize,
        limit: usize,
    ) -> Result<DiagnosticPage, AppError> {
        self.as_ref().list_diagnostics(filter, offset, limit)
    }
    fn record_diagnostic(&self, diagnostic: &RuntimeDiagnostic) -> Result<(), AppError> {
        self.as_ref().record_diagnostic(diagnostic)
    }
    fn clear_diagnostics(
        &self,
        filter: &DiagnosticFilter,
        confirmed: bool,
    ) -> Result<usize, AppError> {
        self.as_ref().clear_diagnostics(filter, confirmed)
    }
    fn recovery_revision(&self) -> Result<u64, AppError> {
        self.as_ref().recovery_revision()
    }
    fn recovery_record(
        &self,
    ) -> Result<Option<crate::configuration_recovery::RecoveryRecord>, AppError> {
        self.as_ref().recovery_record()
    }
    fn begin_recovery(
        &self,
        intent: &crate::configuration_recovery::RecoveryIntent,
    ) -> Result<(), AppError> {
        self.as_ref().begin_recovery(intent)
    }
    fn commit_recovery(&self, id: &str) -> Result<(), AppError> {
        self.as_ref().commit_recovery(id)
    }
    fn clear_recovery(&self, id: &str) -> Result<(), AppError> {
        self.as_ref().clear_recovery(id)
    }
    fn load(&self) -> Result<PersistedConfiguration, AppError> {
        self.as_ref().load()
    }
    #[cfg(test)]
    fn save(&self, configuration: &PersistedConfiguration) -> Result<(), AppError> {
        self.as_ref().save(configuration)
    }
    fn load_mode(&self) -> Result<RuntimeMode, AppError> {
        self.as_ref().load_mode()
    }
    fn save_mode(&self, mode: RuntimeMode) -> Result<(), AppError> {
        self.as_ref().save_mode(mode)
    }
}

fn write_configuration(
    connection: &Connection,
    configuration: &PersistedConfiguration,
) -> Result<(), AppError> {
    configuration.validate()?;
    let json =
        serde_json::to_string(configuration).map_err(|_| AppError::storage("配置无法序列化"))?;
    connection.execute(
        "INSERT INTO configuration (id, document_json) VALUES (1, ?1) ON CONFLICT(id) DO UPDATE SET document_json = excluded.document_json",
        [json],
    ).map_err(storage_error)?;
    prune_diagnostics(
        connection,
        configuration.settings.diagnostic_retention,
        now_ms()?,
    )
    .map(|_| ())
}

#[path = "store_recovery.rs"]
mod recovery;

fn read_configuration(connection: &Connection) -> Result<PersistedConfiguration, AppError> {
    let json: Option<String> = connection
        .query_row(
            "SELECT document_json FROM configuration WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage_error)?;
    let Some(json) = json else {
        return Ok(PersistedConfiguration::default());
    };
    let mut configuration: PersistedConfiguration =
        serde_json::from_str(&json).map_err(|_| AppError::storage("已保存的配置无法读取"))?;
    configuration.migrate_v1()?;
    Ok(configuration)
}

#[path = "store_migrations.rs"]
mod migrations;
use migrations::migrate;
#[cfg(test)]
pub(crate) use migrations::migrate_with_limit;

#[path = "store_diagnostics.rs"]
mod diagnostics;
#[path = "store_proxy_ownership.rs"]
#[cfg(any(windows, test))]
mod ownership;

fn storage_error(_: rusqlite::Error) -> AppError {
    AppError::storage("本地配置数据库操作失败")
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
