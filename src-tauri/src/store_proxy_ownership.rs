use super::*;
use crate::system_proxy::{ProxyOwnership, ProxyOwnershipStore};

impl ProxyOwnershipStore for SqliteConfigurationStore {
    fn load_ownership(&self) -> Result<Option<ProxyOwnership>, AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::storage("代理恢复记录锁不可用"))?;
        let json: Option<String> = connection
            .query_row(
                "SELECT record_json FROM proxy_ownership WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        json.map(|value| {
            serde_json::from_str(&value).map_err(|_| AppError::storage("代理恢复记录已损坏"))
        })
        .transpose()
    }

    fn save_ownership(&self, record: &ProxyOwnership) -> Result<(), AppError> {
        let json = serde_json::to_string(record)
            .map_err(|_| AppError::storage("代理恢复记录无法序列化"))?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::storage("代理恢复记录锁不可用"))?;
        connection
            .execute(
                "INSERT INTO proxy_ownership (id, record_json) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET record_json = excluded.record_json",
                [json],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn clear_ownership(&self) -> Result<(), AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::storage("代理恢复记录锁不可用"))?;
        connection
            .execute("DELETE FROM proxy_ownership WHERE id = 1", [])
            .map_err(storage_error)?;
        Ok(())
    }
}

impl ProxyOwnershipStore for Arc<SqliteConfigurationStore> {
    fn load_ownership(&self) -> Result<Option<ProxyOwnership>, AppError> {
        self.as_ref().load_ownership()
    }

    fn save_ownership(&self, record: &ProxyOwnership) -> Result<(), AppError> {
        self.as_ref().save_ownership(record)
    }

    fn clear_ownership(&self) -> Result<(), AppError> {
        self.as_ref().clear_ownership()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_proxy::ProxySettings;

    #[test]
    fn migrates_existing_version_one_database_and_keeps_ownership_after_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.sqlite3");
        {
            let connection = Connection::open(&path).unwrap();
            connection.execute_batch(
                "CREATE TABLE configuration (id INTEGER PRIMARY KEY, document_json TEXT NOT NULL);
                 CREATE TABLE runtime_diagnostics (id TEXT PRIMARY KEY, created_at_ms INTEGER NOT NULL,
                     severity TEXT NOT NULL, summary TEXT NOT NULL);
                 PRAGMA user_version = 1;",
            ).unwrap();
        }
        let record = ProxyOwnership {
            original: ProxySettings {
                enabled: Some(0),
                server: None,
                bypass: None,
                auto_config_url: None,
                auto_detect: None,
            },
            expected: ProxySettings {
                enabled: Some(1),
                server: Some("127.0.0.1:18080".into()),
                bypass: None,
                auto_config_url: None,
                auto_detect: None,
            },
            pending_expected: None,
            owner_token: "test-token".into(),
        };
        SqliteConfigurationStore::open(&path)
            .unwrap()
            .save_ownership(&record)
            .unwrap();
        let reopened = SqliteConfigurationStore::open(&path).unwrap();
        assert_eq!(reopened.load_ownership().unwrap(), Some(record));
        reopened.clear_ownership().unwrap();
        assert_eq!(reopened.load_ownership().unwrap(), None);
    }
}
