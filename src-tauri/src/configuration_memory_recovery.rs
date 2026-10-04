use super::*;
use crate::configuration_recovery::{recovery_error, RecoveryIntent, RecoveryRecord};

impl ConfigurationStore for Arc<MemoryStore> {
    fn load(&self) -> Result<PersistedConfiguration, AppError> {
        Ok(self.value.lock().unwrap().clone())
    }

    fn save(&self, candidate: &PersistedConfiguration) -> Result<(), AppError> {
        if *self.fail_save.lock().unwrap() {
            return Err(AppError::storage("测试写入失败"));
        }
        *self.value.lock().unwrap() = candidate.clone();
        self.recovery.lock().unwrap().0 += 1;
        Ok(())
    }
    fn load_mode(&self) -> Result<RuntimeMode, AppError> {
        Ok(*self.mode.lock().unwrap())
    }
    fn save_mode(&self, mode: RuntimeMode) -> Result<(), AppError> {
        if *self.fail_save.lock().unwrap() {
            return Err(AppError::storage("测试写入失败"));
        }
        *self.mode.lock().unwrap() = mode;
        Ok(())
    }

    fn recovery_revision(&self) -> Result<u64, AppError> {
        Ok(self.recovery.lock().unwrap().0)
    }
    fn recovery_record(&self) -> Result<Option<RecoveryRecord>, AppError> {
        Ok(self.recovery.lock().unwrap().1.clone())
    }
    fn begin_recovery(&self, intent: &RecoveryIntent) -> Result<(), AppError> {
        intent.validate()?;
        let mut recovery = self.recovery.lock().unwrap();
        if recovery.1.is_some()
            || recovery.0 != intent.previous_revision
            || self.load()? != intent.previous
        {
            return Err(recovery_error());
        }
        recovery.1 = Some(RecoveryRecord {
            intent: intent.clone(),
            committed: false,
        });
        Ok(())
    }
    fn commit_recovery(&self, id: &str) -> Result<(), AppError> {
        if *self.fail_save.lock().unwrap() {
            return Err(AppError::storage("测试写入失败"));
        }
        let mut recovery = self.recovery.lock().unwrap();
        let record = recovery.1.as_mut().ok_or_else(recovery_error)?;
        if record.intent.transaction_id != id {
            return Err(recovery_error());
        }
        *self.value.lock().unwrap() = record.intent.candidate.clone();
        record.committed = true;
        recovery.0 = record.intent.next_revision;
        Ok(())
    }
    fn clear_recovery(&self, id: &str) -> Result<(), AppError> {
        let mut recovery = self.recovery.lock().unwrap();
        if recovery
            .1
            .as_ref()
            .is_some_and(|record| record.intent.transaction_id != id)
        {
            return Err(recovery_error());
        }
        recovery.1 = None;
        Ok(())
    }
}
