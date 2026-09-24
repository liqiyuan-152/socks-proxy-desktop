use crate::{
    error::AppError,
    models::{PersistedConfiguration, RetentionPolicy},
};
use rusqlite::{Connection, OptionalExtension};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

const DATABASE_SCHEMA_VERSION: i64 = 2;
use diagnostics::{now_ms, prune_diagnostics};
pub use diagnostics::{DiagnosticFilter, DiagnosticPage, RuntimeDiagnostic};

pub fn diagnostic_now_ms() -> Result<i64, AppError> {
    now_ms()
}

pub trait ConfigurationStore: Send + Sync {
    fn load(&self) -> Result<PersistedConfiguration, AppError>;
    fn save(&self, configuration: &PersistedConfiguration) -> Result<(), AppError>;
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
}

impl ConfigurationStore for Arc<SqliteConfigurationStore> {
    fn load(&self) -> Result<PersistedConfiguration, AppError> {
        self.as_ref().load()
    }
    fn save(&self, configuration: &PersistedConfiguration) -> Result<(), AppError> {
        self.as_ref().save(configuration)
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
    let configuration: PersistedConfiguration =
        serde_json::from_str(&json).map_err(|_| AppError::storage("已保存的配置无法读取"))?;
    configuration.validate()?;
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
mod tests {
    use super::*;
    use crate::models::{AppSettings, ProxyProfile, ProxyProtocol, RoutingRule};
    use rusqlite::params;

    fn configuration() -> PersistedConfiguration {
        PersistedConfiguration {
            schema_version: crate::models::CONFIG_SCHEMA_VERSION,
            profiles: vec![ProxyProfile {
                id: "profile-stable-id".into(),
                name: "Primary".into(),
                protocol: ProxyProtocol::Socks5,
                host: "127.0.0.1".into(),
                port: 1080,
                authentication_enabled: false,
                credential_ref: None,
                enabled: true,
            }],
            rules: vec![RoutingRule {
                id: "rule-stable-id".into(),
                name: "Internal".into(),
                matcher: crate::models::RuleMatcher::DomainSuffix,
                target: "example.com".into(),
                port_start: None,
                port_end: None,
                action: crate::models::RuleAction::Direct,
                enabled: true,
            }],
            active_profile_id: Some("profile-stable-id".into()),
            settings: AppSettings {
                launch_at_login: true,
                diagnostic_retention: RetentionPolicy::Days90,
            },
        }
    }

    #[test]
    fn migrates_empty_database_and_persists_configuration_across_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.sqlite3");
        let expected = configuration();
        {
            let store = SqliteConfigurationStore::open(&path).unwrap();
            store.save(&expected).unwrap();
            assert_eq!(store.load().unwrap(), expected);
        }
        let reopened = SqliteConfigurationStore::open(&path).unwrap();
        assert_eq!(reopened.load().unwrap(), expected);
    }

    #[test]
    fn migration_sets_supported_schema_version_and_defaults_settings() {
        let store = SqliteConfigurationStore::open_in_memory().unwrap();
        let connection = store.connection.lock().unwrap();
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, DATABASE_SCHEMA_VERSION);
        drop(connection);
        assert_eq!(
            store.load().unwrap().settings.diagnostic_retention,
            RetentionPolicy::Days30
        );
    }

    #[test]
    fn retention_applies_time_windows_and_permanent_capacity() {
        let store = SqliteConfigurationStore::open_in_memory().unwrap();
        let now = now_ms().unwrap();
        {
            let connection = store.connection.lock().unwrap();
            for (index, days_ago) in [0, 20, 60, 120].iter().enumerate() {
                connection
                    .execute(
                        "INSERT INTO runtime_diagnostics VALUES (?1, ?2, 'info', 'runtime event')",
                        params![
                            format!("diag-{index}"),
                            now - days_ago * 24 * 60 * 60 * 1000
                        ],
                    )
                    .unwrap();
            }
        }
        assert_eq!(
            store
                .apply_diagnostic_retention(RetentionPolicy::Days90, now)
                .unwrap(),
            1
        );
        assert_eq!(
            store
                .apply_diagnostic_retention(RetentionPolicy::Days30, now)
                .unwrap(),
            1
        );
        assert_eq!(
            store
                .apply_diagnostic_retention(RetentionPolicy::Days7, now)
                .unwrap(),
            1
        );
        assert_eq!(store.diagnostic_count().unwrap(), 1);
    }

    #[test]
    fn permanent_retention_caps_records_during_insert() {
        let store = SqliteConfigurationStore::open_in_memory().unwrap();
        let mut config = configuration();
        config.settings.diagnostic_retention = RetentionPolicy::Permanent;
        store.save(&config).unwrap();
        let now = now_ms().unwrap();
        {
            let connection = store.connection.lock().unwrap();
            connection
                .execute(
                    "WITH RECURSIVE seq(n) AS (
                    SELECT 0 UNION ALL SELECT n + 1 FROM seq WHERE n < 99999
                ) INSERT INTO runtime_diagnostics (id, created_at_ms, severity, summary)
                  SELECT 'diag-' || n, ?1, 'info', 'runtime event' FROM seq",
                    [now - 1],
                )
                .unwrap();
        }
        store
            .record_diagnostic(&RuntimeDiagnostic {
                id: "newest".into(),
                created_at_ms: now,
                severity: "info".into(),
                summary: "runtime event".into(),
            })
            .unwrap();
        assert_eq!(store.diagnostic_count().unwrap(), 100_000);
        let connection = store.connection.lock().unwrap();
        let newest: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM runtime_diagnostics WHERE id = 'newest'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(newest, 1);
    }

    #[test]
    fn invalid_save_preserves_existing_configuration() {
        let store = SqliteConfigurationStore::open_in_memory().unwrap();
        let expected = configuration();
        store.save(&expected).unwrap();
        let mut invalid = expected.clone();
        invalid.profiles[0].port = 0;
        assert_eq!(
            store.save(&invalid).unwrap_err().fields[0].field,
            "profiles[0].port"
        );
        assert_eq!(store.load().unwrap(), expected);
    }
}
