use super::*;
use crate::configuration_recovery::{recovery_error, RecoveryIntent, RecoveryRecord};

impl SqliteConfigurationStore {
    pub(crate) fn recovery_revision(&self) -> Result<u64, AppError> {
        let connection = self.connection.lock().map_err(|_| recovery_error())?;
        read_revision(&connection)
    }

    pub(crate) fn recovery_record(&self) -> Result<Option<RecoveryRecord>, AppError> {
        let connection = self.connection.lock().map_err(|_| recovery_error())?;
        read_record(&connection)
    }

    pub(crate) fn begin_recovery(&self, intent: &RecoveryIntent) -> Result<(), AppError> {
        intent.validate()?;
        let mut connection = self.connection.lock().map_err(|_| recovery_error())?;
        let transaction = connection.transaction().map_err(storage_error)?;
        if read_record(&transaction)?.is_some()
            || read_revision(&transaction)? != intent.previous_revision
            || read_configuration(&transaction)? != intent.previous
        {
            return Err(recovery_error());
        }
        let json = serde_json::to_string(intent).map_err(|_| recovery_error())?;
        transaction
            .execute(
                "INSERT INTO configuration_recovery (id, intent_json, committed) VALUES (1, ?1, 0)",
                [json],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)
    }

    pub(crate) fn commit_recovery(&self, transaction_id: &str) -> Result<(), AppError> {
        self.commit_recovery_with_hook(transaction_id, || Ok(()))
    }

    fn commit_recovery_with_hook(
        &self,
        transaction_id: &str,
        after_configuration: impl FnOnce() -> Result<(), AppError>,
    ) -> Result<(), AppError> {
        let mut connection = self.connection.lock().map_err(|_| recovery_error())?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let record = read_record(&transaction)?.ok_or_else(recovery_error)?;
        if record.intent.transaction_id != transaction_id {
            return Err(recovery_error());
        }
        let intent = &record.intent;
        let expected = if record.committed {
            (&intent.candidate, intent.next_revision)
        } else {
            (&intent.previous, intent.previous_revision)
        };
        if read_configuration(&transaction)? != *expected.0
            || read_revision(&transaction)? != expected.1
        {
            return Err(recovery_error());
        }
        if record.committed {
            return Ok(());
        }
        write_configuration(&transaction, &intent.candidate)?;
        after_configuration()?;
        transaction
            .execute(
                "UPDATE configuration_commit SET revision = ?1 WHERE id = 1",
                [intent.next_revision],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                "UPDATE configuration_recovery SET committed = 1 WHERE id = 1",
                [],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)
    }

    /// Caller must finish side-effect cleanup first. A failed or interrupted
    /// cleanup deliberately leaves the record available for a repeat attempt.
    pub(crate) fn clear_recovery(&self, transaction_id: &str) -> Result<(), AppError> {
        let mut connection = self.connection.lock().map_err(|_| recovery_error())?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let Some(record) = read_record(&transaction)? else {
            return Ok(());
        };
        if record.intent.transaction_id != transaction_id {
            return Err(recovery_error());
        }
        let expected = if record.committed {
            (&record.intent.candidate, record.intent.next_revision)
        } else {
            (&record.intent.previous, record.intent.previous_revision)
        };
        if read_configuration(&transaction)? != *expected.0
            || read_revision(&transaction)? != expected.1
        {
            return Err(recovery_error());
        }
        transaction
            .execute("DELETE FROM configuration_recovery WHERE id = 1", [])
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)
    }
}

fn read_revision(connection: &Connection) -> Result<u64, AppError> {
    connection
        .query_row(
            "SELECT revision FROM configuration_commit WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(storage_error)
}

pub(super) fn read_record(connection: &Connection) -> Result<Option<RecoveryRecord>, AppError> {
    let record: Option<(String, bool)> = connection
        .query_row(
            "SELECT intent_json, committed FROM configuration_recovery WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(storage_error)?;
    record
        .map(|(json, committed)| {
            let intent: RecoveryIntent =
                serde_json::from_str(&json).map_err(|_| recovery_error())?;
            intent.validate()?;
            Ok(RecoveryRecord { intent, committed })
        })
        .transpose()
}

#[cfg(test)]
#[path = "store_recovery_tests.rs"]
mod tests;
