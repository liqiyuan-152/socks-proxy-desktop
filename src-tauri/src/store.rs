use crate::{
    error::AppError,
    models::{PersistedConfiguration, RetentionPolicy, RuntimeMode},
};
use rusqlite::{Connection, OptionalExtension};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

const DATABASE_SCHEMA_VERSION: i64 = 3;
use diagnostics::{now_ms, prune_diagnostics};
pub use diagnostics::{DiagnosticFilter, DiagnosticPage, RuntimeDiagnostic};

pub fn diagnostic_now_ms() -> Result<i64, AppError> {
    now_ms()
}

pub trait ConfigurationStore: Send + Sync {
    fn load(&self) -> Result<PersistedConfiguration, AppError>;
    fn save(&self, configuration: &PersistedConfiguration) -> Result<(), AppError>;
    fn load_mode(&self) -> Result<RuntimeMode, AppError>;
    fn save_mode(&self, mode: RuntimeMode) -> Result<(), AppError>;
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
    fn load(&self) -> Result<PersistedConfiguration, AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::storage("配置存储锁不可用"))?;
        read_configuration(&connection)
    }

    fn save(&self, configuration: &PersistedConfiguration) -> Result<(), AppError> {
        configuration.validate()?;
        let json = serde_json::to_string(configuration)
            .map_err(|_| AppError::storage("配置无法序列化"))?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| AppError::storage("配置存储锁不可用"))?;
        let transaction = connection.transaction().map_err(storage_error)?;
        transaction
            .execute(
                "INSERT INTO configuration (id, document_json) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET document_json = excluded.document_json",
                [json],
            )
            .map_err(storage_error)?;
        prune_diagnostics(
            &transaction,
            configuration.settings.diagnostic_retention,
            now_ms()?,
        )?;
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
    fn load(&self) -> Result<PersistedConfiguration, AppError> {
        self.as_ref().load()
    }
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

fn migrate(connection: &Connection) -> Result<(), AppError> {
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(storage_error)?;
    if version > DATABASE_SCHEMA_VERSION {
        return Err(AppError::storage("数据库版本高于当前应用支持的版本"));
    }
    if version == 0 {
        let transaction = connection.unchecked_transaction().map_err(storage_error)?;
        transaction
            .execute_batch(
                "CREATE TABLE configuration (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                document_json TEXT NOT NULL
            );
            CREATE TABLE runtime_diagnostics (
                id TEXT PRIMARY KEY,
                created_at_ms INTEGER NOT NULL,
                severity TEXT NOT NULL,
                summary TEXT NOT NULL
            );
            CREATE INDEX runtime_diagnostics_created_at ON runtime_diagnostics(created_at_ms DESC);
            PRAGMA user_version = 1;",
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
    }
    if version <= 1 {
        connection
            .execute_batch(
                "CREATE TABLE proxy_ownership (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                record_json TEXT NOT NULL
            );
            PRAGMA user_version = 2;",
            )
            .map_err(storage_error)?;
    }
    if version <= 2 {
        connection
            .execute_batch(
                "CREATE TABLE selected_mode (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    mode TEXT NOT NULL CHECK (mode IN ('rules', 'global', 'direct'))
                );
                PRAGMA user_version = 3;",
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

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
