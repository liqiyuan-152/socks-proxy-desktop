use super::{rollback_failed, ConfigurationService};
use crate::{
    china_rules::ChinaRuleSets, credentials::ProxyCredential, error::AppError,
    models::PersistedConfiguration,
};

impl ConfigurationService {
    pub(super) fn commit(
        &self,
        current: &PersistedConfiguration,
        candidate: &PersistedConfiguration,
    ) -> Result<(), AppError> {
        self.commit_with_rollback(current, candidate, || Ok(()))
    }

    pub(super) fn commit_with_rollback(
        &self,
        current: &PersistedConfiguration,
        candidate: &PersistedConfiguration,
        rollback_side_effects: impl FnOnce() -> Result<(), AppError>,
    ) -> Result<(), AppError> {
        let validation = (|| {
            candidate.validate()?;
            if candidate.china_direct_enabled {
                let root = self
                    .china_rule_root
                    .as_deref()
                    .ok_or_else(|| AppError::unavailable("此平台未提供国内直连规则集"))?;
                ChinaRuleSets::verify(root)?;
            }
            Ok(())
        })();
        if let Err(error) = validation {
            rollback_side_effects()?;
            return Err(error);
        }
        let previous_snapshot = self.runtime.snapshot();
        if let Err(error) = self.runtime.apply_configuration(current, candidate) {
            rollback_side_effects()?;
            return Err(error);
        }
        if let Err(error) = self.store.save(candidate) {
            // Each recovery must run even if the other one fails.
            let effects = rollback_side_effects();
            let runtime = self
                .runtime
                .restore_configuration(current, &previous_snapshot);
            if effects.is_err() || runtime.is_err() {
                return Err(rollback_failed());
            }
            return Err(error);
        }
        self.configuration_revision
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.runtime.confirm_configuration();
        Ok(())
    }

    pub(super) fn restore_credential(
        &self,
        id: &str,
        previous: Option<ProxyCredential>,
    ) -> Result<(), AppError> {
        let result = match previous {
            Some(secret) => self
                .credentials
                .replace(id, &secret.username, &secret.password),
            None => self.credentials.delete(id),
        };
        result.map_err(|_| rollback_failed())
    }
}
