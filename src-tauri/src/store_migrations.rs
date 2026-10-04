use super::{storage_error, AppError, Connection, DATABASE_SCHEMA_VERSION};

const STEPS: [&str; DATABASE_SCHEMA_VERSION as usize] = [
    "CREATE TABLE configuration (
        id INTEGER PRIMARY KEY CHECK (id = 1), document_json TEXT NOT NULL
     );
     CREATE TABLE runtime_diagnostics (
        id TEXT PRIMARY KEY, created_at_ms INTEGER NOT NULL,
        severity TEXT NOT NULL, summary TEXT NOT NULL
     );
     CREATE INDEX runtime_diagnostics_created_at ON runtime_diagnostics(created_at_ms DESC);",
    "CREATE TABLE proxy_ownership (
        id INTEGER PRIMARY KEY CHECK (id = 1), record_json TEXT NOT NULL
     );",
    "CREATE TABLE selected_mode (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        mode TEXT NOT NULL CHECK (mode IN ('rules', 'global', 'direct'))
     );",
    "CREATE TABLE configuration_recovery (
        id INTEGER PRIMARY KEY CHECK (id = 1), intent_json TEXT NOT NULL,
        committed INTEGER NOT NULL CHECK (committed IN (0, 1))
     );
     CREATE TABLE configuration_commit (
        id INTEGER PRIMARY KEY CHECK (id = 1), revision INTEGER NOT NULL CHECK (revision >= 0)
     );
     INSERT INTO configuration_commit (id, revision) VALUES (1, 0);",
];

pub(super) fn migrate(connection: &Connection) -> Result<(), AppError> {
    migrate_with_hook(connection, |_| Ok(()))
}

fn migrate_with_hook(
    connection: &Connection,
    after_ddl: impl Fn(i64) -> Result<(), AppError>,
) -> Result<(), AppError> {
    migrate_with_limit(connection, DATABASE_SCHEMA_VERSION, after_ddl)
}

// The compatibility rehearsal calls the same version gate with the previous
// binary's supported version. Production always supplies the current version.
pub(crate) fn migrate_with_limit(
    connection: &Connection,
    supported_version: i64,
    after_ddl: impl Fn(i64) -> Result<(), AppError>,
) -> Result<(), AppError> {
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(storage_error)?;
    if version > supported_version {
        return Err(AppError::storage("数据库版本高于当前应用支持的版本"));
    }
    for next_version in version + 1..=supported_version {
        let transaction = connection.unchecked_transaction().map_err(storage_error)?;
        transaction
            .execute_batch(STEPS[(next_version - 1) as usize])
            .map_err(storage_error)?;
        after_ddl(next_version)?;
        transaction
            .pragma_update(None, "user_version", next_version)
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{ConfigurationStore, SqliteConfigurationStore};

    #[test]
    #[ignore = "invoked by migration interruption test in an isolated process"]
    fn interrupted_migration_child() {
        let path = std::env::var("MIGRATION_TEST_DATABASE").unwrap();
        let target: i64 = std::env::var("MIGRATION_TEST_VERSION")
            .unwrap()
            .parse()
            .unwrap();
        let connection = Connection::open(path).unwrap();
        let _ = migrate_with_hook(&connection, |version| {
            if version == target {
                // Exit without dropping Transaction/Connection, like an abrupt
                // process interruption. SQLite must recover its durable journal.
                std::process::exit(87);
            }
            Ok(())
        });
        panic!("interruption point was not reached");
    }

    #[test]
    fn interrupted_ddl_rolls_back_each_step_and_reopens_without_losing_configuration() {
        for target in 1..=DATABASE_SCHEMA_VERSION {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("config.sqlite3");
            let connection = Connection::open(&path).unwrap();
            // Commit the preceding steps, then simulate a returned failure to
            // establish an authentic previous-version database.
            assert!(migrate_with_hook(&connection, |version| {
                if version == target {
                    Err(AppError::storage("injected"))
                } else {
                    Ok(())
                }
            })
            .is_err());
            let mut configuration = crate::models::PersistedConfiguration::default();
            configuration.settings.launch_at_login = true;
            if target > 1 {
                connection
                    .execute(
                        "INSERT INTO configuration VALUES (1, ?1)",
                        [serde_json::to_string(&configuration).unwrap()],
                    )
                    .unwrap();
            }
            drop(connection);
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "store::migrations::tests::interrupted_migration_child",
                    "--ignored",
                ])
                .env("MIGRATION_TEST_DATABASE", &path)
                .env("MIGRATION_TEST_VERSION", target.to_string())
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(87), "{output:?}");
            let connection = Connection::open(&path).unwrap();
            let version: i64 = connection
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .unwrap();
            assert_eq!(version, target - 1);
            let table = [
                "configuration",
                "proxy_ownership",
                "selected_mode",
                "configuration_recovery",
            ][(target - 1) as usize];
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 0);
            drop(connection);
            let reopened = SqliteConfigurationStore::open(&path).unwrap();
            if target > 1 {
                assert_eq!(reopened.load().unwrap(), configuration);
            }
            drop(reopened);
            assert!(SqliteConfigurationStore::open(&path).is_ok());
        }
    }

    #[test]
    fn future_database_is_rejected_without_mutation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("future.sqlite3");
        let connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "user_version", DATABASE_SCHEMA_VERSION + 1)
            .unwrap();
        drop(connection);
        let before = std::fs::read(&path).unwrap();
        assert!(SqliteConfigurationStore::open(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}
