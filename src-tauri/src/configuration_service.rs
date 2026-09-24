use crate::{
    china_rules::ChinaRuleSets,
    credentials::{apply_credential_update, CredentialStore, CredentialUpdate, ProxyCredential},
    error::{AppError, FieldError},
    models::{
        AppSettings, PersistedConfiguration, ProxyProfile, ProxyProtocol, RoutingRule, RuntimeMode,
    },
    observability::ActiveConnectionsSnapshot,
    routing::CompiledRules,
    runtime::{RuntimeCoordinator, RuntimeSnapshot},
    startup::StartupAdapter,
    store::ConfigurationStore,
    transfer::export_configuration_json,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Mutex};
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
        }
    }

    pub fn list_profiles(&self) -> Result<Vec<ProfileView>, AppError> {
        Ok(self
            .store
            .load()?
            .profiles
            .iter()
            .map(ProfileView::from)
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
        Ok(ProfileView::from(&profile))
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

    pub fn replace_rules(&self, rules: Vec<RoutingRule>) -> Result<(), AppError> {
        let _guard = self.lock()?;
        let current = self.store.load()?;
        let mut candidate = current.clone();
        candidate.rules = rules;
        candidate.legacy_unresolved_rule_ids.retain(|id| {
            candidate.rules.iter().any(|rule| {
                rule.id == *id
                    && rule.action == crate::models::RuleAction::Proxy
                    && rule.proxy_profile_id.is_none()
            })
        });
        CompiledRules::compile(&candidate)?;
        self.commit(&current, &candidate)
    }

    pub fn reorder_rules(&self, ids: &[String]) -> Result<(), AppError> {
        let _guard = self.lock()?;
        let current = self.store.load()?;
        if ids.len() != current.rules.len() {
            return Err(field_error("rule_ids", "排序必须包含全部规则标识"));
        }
        let mut remaining = current.rules.clone();
        let mut sorted = Vec::with_capacity(ids.len());
        for id in ids {
            let Some(index) = remaining.iter().position(|rule| &rule.id == id) else {
                return Err(field_error("rule_ids", "排序包含重复或不存在的规则标识"));
            };
            sorted.push(remaining.remove(index));
        }
        let mut candidate = current.clone();
        candidate.rules = sorted;
        CompiledRules::compile(&candidate)?;
        self.commit(&current, &candidate)
    }

    pub fn update_settings(&self, settings: AppSettings) -> Result<AppSettings, AppError> {
        let _guard = self.lock()?;
        let current = self.store.load()?;
        let previous_startup = self.startup.is_enabled()?;
        if settings.launch_at_login != previous_startup {
            self.startup.set_enabled(settings.launch_at_login)?;
        }
        let mut candidate = current.clone();
        candidate.settings = settings;
        self.commit_with_rollback(&current, &candidate, || {
            self.startup
                .set_enabled(previous_startup)
                .map_err(|_| rollback_failed())
        })?;
        self.settings()
    }

    pub fn export(&self) -> Result<String, AppError> {
        export_configuration_json(&self.store.load()?)
    }

    fn commit(
        &self,
        current: &PersistedConfiguration,
        candidate: &PersistedConfiguration,
    ) -> Result<(), AppError> {
        self.commit_with_rollback(current, candidate, || Ok(()))
    }

    fn commit_with_rollback(
        &self,
        current: &PersistedConfiguration,
        candidate: &PersistedConfiguration,
        rollback_side_effects: impl FnOnce() -> Result<(), AppError>,
    ) -> Result<(), AppError> {
        candidate.validate()?;
        if candidate.china_direct_enabled {
            let root = self
                .china_rule_root
                .as_deref()
                .ok_or_else(|| AppError::unavailable("此平台未提供国内直连规则集"))?;
            ChinaRuleSets::verify(root)?;
        }
        let previous_snapshot = self.runtime.snapshot();
        if let Err(error) = self.runtime.apply_configuration(current, candidate) {
            rollback_side_effects()?;
            return Err(error);
        }
        if let Err(error) = self.store.save(candidate) {
            rollback_side_effects()?;
            self.runtime
                .restore_configuration(current, &previous_snapshot)
                .map_err(|_| rollback_failed())?;
            return Err(error);
        }
        self.runtime.confirm_configuration();
        Ok(())
    }

    fn restore_credential(
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

#[path = "configuration_service_import.rs"]
mod import;

#[cfg(test)]
#[path = "configuration_service_tests.rs"]
mod tests;
