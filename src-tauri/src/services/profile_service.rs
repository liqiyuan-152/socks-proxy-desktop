//! 代理档案服务：校验档案、隔离凭据并通过共享事务协调器提交。
use super::{context::ConfigurationContext, interfaces::ProfileService, profile_types::*};
use crate::{
    credentials::{prepare_credential_update, CredentialUpdate},
    domain_errors::ProxyError,
    error::FieldError,
    models::ProxyProfile,
};
use std::sync::Arc;
use uuid::Uuid;

/// 管理代理档案和凭据，不依赖应用 facade 或其他领域服务。
pub struct ProxyProfileService {
    context: Arc<ConfigurationContext>,
}

impl ProxyProfileService {
    pub(crate) fn new(context: Arc<ConfigurationContext>) -> Self {
        Self { context }
    }
}

impl ProfileService for ProxyProfileService {
    #[tracing::instrument(skip_all, level = "debug")]
    fn list_profiles(&self) -> Result<Vec<ProfileView>, ProxyError> {
        let _guard = self.context.lock()?;
        let revision = self.context.store.recovery_revision()?;
        Ok(self
            .context
            .store
            .load()?
            .profiles
            .iter()
            .map(|profile| {
                let mut view = ProfileView::from(profile);
                view.configuration_revision = revision;
                view
            })
            .collect())
    }

    #[tracing::instrument(skip_all, level = "debug")]
    fn get_credential(&self, id: &str) -> Result<ProfileCredentialView, ProxyError> {
        let _guard = self.context.lock()?;
        let configuration = self.context.store.load()?;
        let profile = configuration
            .profiles
            .iter()
            .find(|profile| profile.id == id)
            .ok_or_else(|| ProxyError::NotFound { id: id.into() })?;
        if !profile.authentication_enabled {
            return Err(crate::domain_errors::CredentialError::Disabled.into());
        }
        let credential = crate::credentials::read_profile_credential(
            self.context.credentials.as_ref(),
            profile,
        )?
        .ok_or(crate::domain_errors::CredentialError::Missing)?;
        Ok(ProfileCredentialView {
            username: credential.username,
            password: credential.password,
        })
    }

    #[tracing::instrument(skip_all, level = "debug")]
    fn save_profile(&self, input: ProfileInput) -> Result<ProfileView, ProxyError> {
        let _guard = self.context.mutation_lock()?;
        let current = self.context.store.load()?;
        let mut candidate = current.clone();
        let editing = input.id.is_some();
        let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let existing_index = candidate
            .profiles
            .iter()
            .position(|profile| profile.id == id);
        if input.authentication_enabled
            && matches!(input.credential.as_ref(), Some(CredentialUpdate::Delete))
        {
            return Err(field_error("credential", "启用认证时必须提供或保留凭据"));
        }
        if !input.authentication_enabled
            && matches!(
                input.credential.as_ref(),
                Some(CredentialUpdate::Replace { .. })
            )
        {
            return Err(field_error("credential", "关闭认证时不能替换凭据"));
        }
        if editing && existing_index.is_none() {
            return Err(ProxyError::NotFound { id });
        }
        let mut profile = ProxyProfile {
            credential_ref: input.authentication_enabled.then(|| {
                existing_index
                    .and_then(|index| current.profiles[index].credential_ref.clone())
                    .unwrap_or_else(|| id.clone())
            }),
            id: id.clone(),
            name: input.name,
            protocol: input.protocol,
            host: input.host,
            port: input.port,
            authentication_enabled: input.authentication_enabled,
            enabled: input.enabled,
        };
        if let Some(index) = existing_index {
            candidate.profiles[index] = profile.clone();
        } else {
            candidate.profiles.push(profile.clone());
        }
        candidate.validate()?;
        let staged_secret = prepare_credential_update(
            self.context.credentials.as_ref(),
            &mut profile,
            input.credential,
        )
        .map_err(|error| {
            if error.code == "validation_error" {
                ProxyError::CredentialInvalid(crate::domain_errors::CredentialError::Invalid(
                    crate::domain_errors::ValidationError {
                        fields: error.fields,
                    },
                ))
            } else {
                error.into()
            }
        })?;
        if let Some(index) = existing_index {
            candidate.profiles[index] = profile.clone();
        } else {
            *candidate
                .profiles
                .last_mut()
                .expect("new profile was appended") = profile.clone();
        }
        let staged = staged_secret
            .map(|secret| {
                (
                    profile.credential_ref.clone().expect("staged reference"),
                    secret,
                )
            })
            .into_iter()
            .collect();
        self.context
            .commit_effects(&current, &candidate, staged, None)?;
        let mut view = ProfileView::from(&profile);
        view.configuration_revision = self.context.store.recovery_revision()?;
        Ok(view)
    }

    #[tracing::instrument(skip_all, level = "debug")]
    fn delete_profile(&self, id: &str) -> Result<(), ProxyError> {
        let _guard = self.context.mutation_lock()?;
        let current = self.context.store.load()?;
        let mut candidate = current.clone();
        let Some(index) = candidate
            .profiles
            .iter()
            .position(|profile| profile.id == id)
        else {
            return Err(ProxyError::NotFound { id: id.into() });
        };
        let mut references: Vec<FieldError> = candidate
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.proxy_profile_id.as_deref() == Some(id))
            .map(|(index, rule)| FieldError {
                field: format!("rules[{index}].proxy_profile_id"),
                message: format!("规则「{}」仍引用此代理", rule.name),
            })
            .collect();
        if candidate.active_profile_id.as_deref() == Some(id) {
            references.push(FieldError {
                field: "default_profile_id".into(),
                message: "默认代理仍引用此档案".into(),
            });
        }
        if !references.is_empty() {
            return Err(ProxyError::InUse {
                references: references
                    .into_iter()
                    .map(|reference| crate::domain_errors::ReferenceLocation {
                        field: reference.field,
                        description: reference.message,
                    })
                    .collect(),
            });
        }
        candidate.profiles.remove(index);
        candidate.validate()?;
        self.context.commit(&current, &candidate)?;
        Ok(())
    }

    #[tracing::instrument(skip_all, level = "debug")]
    fn select_profile(&self, id: Option<String>) -> Result<(), ProxyError> {
        let _guard = self.context.mutation_lock()?;
        let current = self.context.store.load()?;
        let mut candidate = current.clone();
        candidate.active_profile_id = id;
        candidate.validate()?;
        self.context
            .commit(&current, &candidate)
            .map_err(Into::into)
    }
}

fn field_error(field: &str, message: &str) -> ProxyError {
    crate::domain_errors::CredentialError::Invalid(crate::domain_errors::ValidationError {
        fields: vec![FieldError {
            field: field.into(),
            message: message.into(),
        }],
    })
    .into()
}

#[cfg(test)]
#[path = "profile_service_tests.rs"]
mod tests;
