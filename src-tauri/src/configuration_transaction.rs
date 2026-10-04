use super::{rollback_failed, ConfigurationService};
use crate::{
    china_rules::ChinaRuleSets, configuration_recovery::StartupChange,
    credentials::ProxyCredential, error::AppError, models::PersistedConfiguration,
};

impl ConfigurationService {
    pub(super) fn commit(
        &self,
        current: &PersistedConfiguration,
        candidate: &PersistedConfiguration,
    ) -> Result<(), AppError> {
        self.commit_durable(current, candidate, vec![], None)
    }

    pub(super) fn commit_effects(
        &self,
        current: &PersistedConfiguration,
        candidate: &PersistedConfiguration,
        staged: Vec<(String, ProxyCredential)>,
        startup: Option<StartupChange>,
    ) -> Result<(), AppError> {
        self.commit_durable(current, candidate, staged, startup)
    }

    pub(crate) fn report_configuration_recovery_issue(&self, error: &AppError) {
        self.runtime.report_configuration_recovery_issue(error);
    }

    pub(crate) fn upgrade_legacy_credentials(&self) -> Result<(), AppError> {
        let _guard = self.mutation_lock()?;
        let current = self.store.load()?;
        let mut candidate = current.clone();
        let mut staged = Vec::new();
        for profile in &mut candidate.profiles {
            if profile.authentication_enabled
                && profile.credential_ref.as_deref() == Some(profile.id.as_str())
            {
                let secret = crate::credentials::prepare_credential_update(
                    self.credentials.as_ref(),
                    profile,
                    None,
                )?
                .expect("legacy credential is staged");
                staged.push((
                    profile.credential_ref.clone().expect("staged reference"),
                    secret,
                ));
            }
        }
        if staged.is_empty() {
            return Ok(());
        }
        self.commit_effects(&current, &candidate, staged, None)
    }

    pub(super) fn startup_change(&self, enabled: bool) -> Result<Option<StartupChange>, AppError> {
        if self.startup.is_enabled()? == enabled {
            return Ok(None);
        }
        self.startup.prepare_change(enabled).map(Some)
    }
}

#[path = "configuration_durable_commit.rs"]
mod durable;
