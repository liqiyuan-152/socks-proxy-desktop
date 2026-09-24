use super::*;
use crate::transfer::parse_import_configuration;
use std::collections::{HashMap, HashSet};

impl ConfigurationService {
    pub fn import(
        &self,
        json: &str,
        updates: HashMap<String, CredentialUpdate>,
    ) -> Result<(), AppError> {
        let _guard = self.lock()?;
        let candidate = parse_import_configuration(json, &updates)?;
        let current = self.store.load()?;
        let previous_startup = self.startup.is_enabled()?;
        let ids: HashSet<String> = current
            .profiles
            .iter()
            .chain(candidate.profiles.iter())
            .map(|profile| profile.id.clone())
            .collect();
        let previous_credentials: Vec<(String, Option<ProxyCredential>)> = ids
            .into_iter()
            .map(|id| self.credentials.get(&id).map(|credential| (id, credential)))
            .collect::<Result<_, _>>()?;

        if let Err(error) = self.stage_import(&current, &candidate, &updates, previous_startup) {
            self.restore_import(previous_credentials, previous_startup)?;
            return Err(error);
        }
        self.commit_with_rollback(&current, &candidate, || {
            self.restore_import(previous_credentials, previous_startup)
        })
    }

    fn stage_import(
        &self,
        current: &PersistedConfiguration,
        candidate: &PersistedConfiguration,
        updates: &HashMap<String, CredentialUpdate>,
        previous_startup: bool,
    ) -> Result<(), AppError> {
        for profile in &candidate.profiles {
            if let Some(CredentialUpdate::Replace { username, password }) = updates.get(&profile.id)
            {
                self.credentials.replace(&profile.id, username, password)?;
            } else if !profile.authentication_enabled {
                self.credentials.delete(&profile.id)?;
            }
        }
        for profile in &current.profiles {
            if !candidate.profiles.iter().any(|new| new.id == profile.id) {
                self.credentials.delete(&profile.id)?;
            }
        }
        if candidate.settings.launch_at_login != previous_startup {
            self.startup
                .set_enabled(candidate.settings.launch_at_login)?;
        }
        Ok(())
    }

    fn restore_import(
        &self,
        previous_credentials: Vec<(String, Option<ProxyCredential>)>,
        previous_startup: bool,
    ) -> Result<(), AppError> {
        let mut failed = false;
        for (id, credential) in previous_credentials {
            failed |= self.restore_credential(&id, credential).is_err();
        }
        if self.startup.is_enabled()? != previous_startup {
            failed |= self.startup.set_enabled(previous_startup).is_err();
        }
        if failed {
            Err(rollback_failed())
        } else {
            Ok(())
        }
    }
}
