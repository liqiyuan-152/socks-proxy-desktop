use super::*;
use crate::transfer::parse_import_configuration;
use std::collections::HashMap;

impl ConfigurationService {
    pub fn import(
        &self,
        json: &str,
        mut updates: HashMap<String, CredentialUpdate>,
    ) -> Result<(), AppError> {
        let _guard = self.mutation_lock()?;
        let mut candidate = parse_import_configuration(json, &updates)?;
        let current = self.store.load()?;
        let startup = self.startup_change(candidate.settings.launch_at_login)?;
        let mut staged = Vec::new();
        for profile in &mut candidate.profiles {
            if let Some(secret) = prepare_credential_update(
                self.credentials.as_ref(),
                profile,
                updates.remove(&profile.id),
            )? {
                staged.push((
                    profile.credential_ref.clone().expect("staged reference"),
                    secret,
                ));
            }
        }
        self.commit_effects(&current, &candidate, staged, startup)
    }
}
