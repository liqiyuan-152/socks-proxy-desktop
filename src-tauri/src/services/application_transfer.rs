use super::application_service::ApplicationService;
use crate::transfer::parse_import_configuration;
use crate::{
    credentials::{prepare_credential_update, CredentialUpdate},
    error::AppError,
};
use std::collections::HashMap;

impl ApplicationService {
    pub(crate) fn import_transaction(
        &self,
        json: &str,
        mut updates: HashMap<String, CredentialUpdate>,
    ) -> Result<(), AppError> {
        let _guard = self.context.mutation_lock()?;
        let mut candidate = parse_import_configuration(json, &updates)?;
        let current = self.context.store.load()?;
        let startup = self
            .context
            .startup_change(candidate.settings.launch_at_login)?;
        let mut staged = Vec::new();
        for profile in &mut candidate.profiles {
            if let Some(secret) = prepare_credential_update(
                self.context.credentials.as_ref(),
                profile,
                updates.remove(&profile.id),
            )? {
                staged.push((
                    profile.credential_ref.clone().expect("staged reference"),
                    secret,
                ));
            }
        }
        self.context
            .commit_effects(&current, &candidate, staged, startup)
    }
}
