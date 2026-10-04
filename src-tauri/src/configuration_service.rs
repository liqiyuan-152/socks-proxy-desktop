use crate::{
    credentials::{apply_credential_update, CredentialStore, CredentialUpdate},
    error::{AppError, FieldError},
    models::{
        AppSettings, PersistedConfiguration, ProxyProfile, ProxyProtocol, RoutingRule, RuntimeMode,
    },
    observability::ActiveConnectionsSnapshot,
    runtime::{RuntimeCoordinator, RuntimeSnapshot},
    startup::StartupAdapter,
    store::ConfigurationStore,
    transfer::export_configuration_json,
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};
use uuid::Uuid;

#[path = "configuration_china.rs"]
mod china;
pub use china::ChinaDirectStatus;

#[derive(Deserialize)]
pub struct ProfileInput {
    pub id: Option<String>,
    pub name: String,
    pub protocol: ProxyProtocol,
    pub host: String,
    pub port: u16,
    pub authentication_enabled: bool,
    pub enabled: bool,
    pub credential: Option<CredentialUpdate>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProfileView {
    pub configuration_revision: u64,
    pub id: String,
    pub name: String,
    pub protocol: ProxyProtocol,
    pub host: String,
    pub port: u16,
    pub authentication_enabled: bool,
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct ProfileCredentialView {
    pub username: String,
    pub password: String,
}

impl From<&ProxyProfile> for ProfileView {
    fn from(value: &ProxyProfile) -> Self {
        Self {
            configuration_revision: 0,
            id: value.id.clone(),
            name: value.name.clone(),
            protocol: value.protocol,
            host: value.host.clone(),
            port: value.port,
            authentication_enabled: value.authentication_enabled,
            enabled: value.enabled,
        }
    }
}

pub struct ConfigurationService {
    store: Box<dyn ConfigurationStore>,
    credentials: Box<dyn CredentialStore>,
    startup: Box<dyn StartupAdapter>,
    runtime: Box<dyn RuntimeCoordinator>,
    china_rule_root: Option<PathBuf>,
    serial: Mutex<()>,
    configuration_revision: AtomicU64,
}

impl ConfigurationService {
    pub fn new(
        store: Box<dyn ConfigurationStore>,
        credentials: Box<dyn CredentialStore>,
        startup: Box<dyn StartupAdapter>,
        runtime: Box<dyn RuntimeCoordinator>,
    ) -> Self {
        Self {
            store,
            credentials,
            startup,
            runtime,
            china_rule_root: None,
            serial: Mutex::new(()),
            configuration_revision: AtomicU64::new(0),
        }
    }

    pub fn list_profiles(&self) -> Result<Vec<ProfileView>, AppError> {
        let _guard = self.lock()?;
        let revision = self.configuration_revision.load(Ordering::Relaxed);
        Ok(self
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

    pub fn list_rules(&self) -> Result<Vec<RoutingRule>, AppError> {
        Ok(self.store.load()?.rules)
    }

    pub fn settings(&self) -> Result<AppSettings, AppError> {
        let mut settings = self.store.load()?.settings;
        settings.launch_at_login = self.startup.is_enabled()?;
        Ok(settings)
    }

    pub fn runtime_snapshot(&self) -> RuntimeSnapshot {
        self.runtime.snapshot()
    }

    pub fn active_connections(&self) -> ActiveConnectionsSnapshot {
        self.runtime.active_connections()
    }

    pub fn request_mode(&self, mode: RuntimeMode) -> Result<RuntimeSnapshot, AppError> {
        let _guard = self.lock()?;
        self.store.save_mode(mode)?;
        self.runtime.request_mode(mode)
    }

    pub fn profile_credential(&self, id: &str) -> Result<ProfileCredentialView, AppError> {
        let _guard = self.lock()?;
        let configuration = self.store.load()?;
        let profile = configuration
            .profiles
            .iter()
            .find(|profile| profile.id == id)
            .ok_or_else(|| not_found("代理档案不存在"))?;
        if !profile.authentication_enabled {
            return Err(AppError::unavailable("此代理未启用认证"));
        }
        let credential = self
            .credentials
            .get(id)?
            .ok_or_else(|| AppError::unavailable("认证凭据缺失，请重新配置"))?;
        Ok(ProfileCredentialView {
            username: credential.username,
            password: credential.password,
        })
    }

    pub fn stop_runtime(&self) -> Result<RuntimeSnapshot, AppError> {
        let _guard = self.lock()?;
        self.store.save_mode(RuntimeMode::Direct)?;
        self.runtime.stop()
    }

    pub fn recover_network(&self) -> Result<RuntimeSnapshot, AppError> {
        let _guard = self.lock()?;
        self.runtime.recover_network()
    }

    pub fn save_profile(&self, input: ProfileInput) -> Result<ProfileView, AppError> {
        let _guard = self.lock()?;
        let current = self.store.load()?;
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
            return Err(not_found("代理档案不存在"));
        }
        let mut profile = ProxyProfile {
            credential_ref: input.authentication_enabled.then(|| id.clone()),
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
        let previous_credential = self.credentials.get(&id)?;
        let changed_credential = !input.authentication_enabled
            || !matches!(
                input.credential.as_ref(),
                None | Some(CredentialUpdate::Preserve)
            );
        apply_credential_update(self.credentials.as_ref(), &mut profile, input.credential)?;
        if let Some(index) = existing_index {
            candidate.profiles[index] = profile.clone();
        } else {
            *candidate
                .profiles
                .last_mut()
                .expect("new profile was appended") = profile.clone();
        }
        self.commit_with_rollback(&current, &candidate, || {
            if changed_credential {
                self.restore_credential(&id, previous_credential)?;
            }
            Ok(())
        })?;
        let mut view = ProfileView::from(&profile);
        view.configuration_revision = self.configuration_revision.load(Ordering::Relaxed);
        Ok(view)
    }

    pub fn delete_profile(&self, id: &str) -> Result<(), AppError> {
        let _guard = self.lock()?;
        let current = self.store.load()?;
        let mut candidate = current.clone();
        let Some(index) = candidate
            .profiles
            .iter()
            .position(|profile| profile.id == id)
        else {
            return Err(not_found("代理档案不存在"));
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
            return Err(AppError::validation(references));
        }
        candidate.profiles.remove(index);
        candidate.validate()?;
        let previous_credential = self.credentials.get(id)?;
        self.credentials.delete(id)?;
        self.commit_with_rollback(&current, &candidate, || {
            self.restore_credential(id, previous_credential)
        })?;
        Ok(())
    }

    pub fn select_profile(&self, id: Option<String>) -> Result<(), AppError> {
        let _guard = self.lock()?;
        let current = self.store.load()?;
        let mut candidate = current.clone();
        candidate.active_profile_id = id;
        candidate.validate()?;
        self.commit(&current, &candidate)
    }

    pub fn update_settings(&self, settings: AppSettings) -> Result<AppSettings, AppError> {
        let _guard = self.lock()?;
        let current = self.store.load()?;
        let previous_startup = self.startup.is_enabled()?;
        let startup_changed = settings.launch_at_login != previous_startup;
        if startup_changed {
            self.startup.set_enabled(settings.launch_at_login)?;
        }
        let mut candidate = current.clone();
        candidate.settings = settings;
        self.commit_with_rollback(&current, &candidate, || {
            if startup_changed {
                self.startup
                    .set_enabled(previous_startup)
                    .map_err(|_| rollback_failed())
            } else {
                Ok(())
            }
        })?;
        self.settings()
    }

    pub fn export(&self) -> Result<String, AppError> {
        export_configuration_json(&self.store.load()?)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, ()>, AppError> {
        self.serial
            .lock()
            .map_err(|_| AppError::storage("配置写入锁不可用"))
    }
}

fn field_error(field: &str, message: &str) -> AppError {
    AppError::validation(vec![FieldError {
        field: field.into(),
        message: message.into(),
    }])
}

fn not_found(message: &str) -> AppError {
    AppError {
        code: "not_found".into(),
        message: message.into(),
        fields: Vec::new(),
    }
}

fn rollback_failed() -> AppError {
    AppError {
        code: "rollback_failed".into(),
        message: "恢复上次配置失败，需要检查运行时与系统代理状态".into(),
        fields: Vec::new(),
    }
}

#[path = "configuration_transaction.rs"]
mod transaction;

#[path = "configuration_rules.rs"]
mod rules;

#[path = "configuration_service_import.rs"]
mod import;

#[cfg(test)]
#[path = "configuration_service_tests.rs"]
mod tests;
