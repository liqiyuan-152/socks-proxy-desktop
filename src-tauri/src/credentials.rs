use crate::{
    error::{AppError, FieldError},
    models::{PersistedConfiguration, ProxyProfile},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const KEYRING_SERVICE: &str = "com.socks-proxy.desktop.profile";

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum CredentialUpdate {
    Preserve,
    Replace { username: String, password: String },
    Delete,
}

pub struct ProxyCredential {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
struct StoredCredential {
    username: String,
    password: String,
}

pub trait CredentialStore: Send + Sync {
    fn get(&self, profile_id: &str) -> Result<Option<ProxyCredential>, AppError>;
    fn replace(&self, profile_id: &str, username: &str, password: &str) -> Result<(), AppError>;
    fn delete(&self, profile_id: &str) -> Result<(), AppError>;
}

pub struct OsCredentialStore;

impl CredentialStore for OsCredentialStore {
    fn get(&self, profile_id: &str) -> Result<Option<ProxyCredential>, AppError> {
        let entry = entry(profile_id)?;
        let value = match entry.get_password() {
            Ok(value) => value,
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(_) => return Err(credential_error()),
        };
        let stored: StoredCredential =
            serde_json::from_str(&value).map_err(|_| credential_error())?;
        Ok(Some(ProxyCredential {
            username: stored.username,
            password: stored.password,
        }))
    }

    fn replace(&self, profile_id: &str, username: &str, password: &str) -> Result<(), AppError> {
        let value = serde_json::to_string(&StoredCredential {
            username: username.into(),
            password: password.into(),
        })
        .map_err(|_| credential_error())?;
        entry(profile_id)?
            .set_password(&value)
            .map_err(|_| credential_error())
    }

    fn delete(&self, profile_id: &str) -> Result<(), AppError> {
        match entry(profile_id)?.delete_password() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(credential_error()),
        }
    }
}

fn entry(profile_id: &str) -> Result<keyring::Entry, AppError> {
    if profile_id.trim().is_empty() {
        return Err(AppError::validation(vec![FieldError {
            field: "id".into(),
            message: "档案标识不能为空".into(),
        }]));
    }
    keyring::Entry::new(KEYRING_SERVICE, profile_id).map_err(|_| credential_error())
}

fn credential_error() -> AppError {
    AppError {
        code: "credential_error".into(),
        message: "系统凭据存储不可用".into(),
        fields: Vec::new(),
    }
}

pub fn apply_credential_update(
    store: &dyn CredentialStore,
    profile: &mut ProxyProfile,
    update: Option<CredentialUpdate>,
) -> Result<(), AppError> {
    match update.unwrap_or(CredentialUpdate::Preserve) {
        CredentialUpdate::Preserve if profile.authentication_enabled => {
            if store.get(&profile.id)?.is_none() {
                return Err(AppError::validation(vec![FieldError {
                    field: "credential".into(),
                    message: "认证凭据缺失，请重新输入".into(),
                }]));
            }
            profile.credential_ref = Some(profile.id.clone());
        }
        CredentialUpdate::Replace { username, password } => {
            if username.trim().is_empty() || password.is_empty() {
                return Err(AppError::validation(vec![FieldError {
                    field: "credential".into(),
                    message: "用户名和密码不能为空".into(),
                }]));
            }
            store.replace(&profile.id, &username, &password)?;
            profile.authentication_enabled = true;
            profile.credential_ref = Some(profile.id.clone());
        }
        CredentialUpdate::Delete | CredentialUpdate::Preserve => {
            store.delete(&profile.id)?;
            profile.authentication_enabled = false;
            profile.credential_ref = None;
        }
    }
    Ok(())
}

pub fn validate_import_credentials(
    configuration: &PersistedConfiguration,
    updates: &HashMap<String, CredentialUpdate>,
) -> Result<(), AppError> {
    let mut fields = Vec::new();
    for (index, profile) in configuration.profiles.iter().enumerate() {
        if profile.authentication_enabled {
            match updates.get(&profile.id) {
                Some(CredentialUpdate::Replace { username, password })
                    if !username.trim().is_empty() && !password.is_empty() => {}
                _ => fields.push(FieldError {
                    field: format!("profiles[{index}].credential"),
                    message: "导入已认证档案时必须重新提供用户名和密码".into(),
                }),
            }
        }
    }
    for id in updates.keys() {
        match configuration
            .profiles
            .iter()
            .find(|profile| &profile.id == id)
        {
            None => fields.push(FieldError {
                field: "credentials".into(),
                message: "凭据指向不存在的代理档案".into(),
            }),
            Some(profile) if !profile.authentication_enabled => fields.push(FieldError {
                field: "credentials".into(),
                message: "未启用认证的档案不能导入凭据".into(),
            }),
            Some(_) => {}
        }
    }
    if fields.is_empty() {
        Ok(())
    } else {
        Err(AppError::validation(fields))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ProxyProtocol;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryCredentials(Mutex<HashMap<String, ProxyCredential>>);

    impl CredentialStore for MemoryCredentials {
        fn get(&self, id: &str) -> Result<Option<ProxyCredential>, AppError> {
            Ok(self.0.lock().unwrap().get(id).map(|item| ProxyCredential {
                username: item.username.clone(),
                password: item.password.clone(),
            }))
        }
        fn replace(&self, id: &str, username: &str, password: &str) -> Result<(), AppError> {
            self.0.lock().unwrap().insert(
                id.into(),
                ProxyCredential {
                    username: username.into(),
                    password: password.into(),
                },
            );
            Ok(())
        }
        fn delete(&self, id: &str) -> Result<(), AppError> {
            self.0.lock().unwrap().remove(id);
            Ok(())
        }
    }

    fn profile() -> ProxyProfile {
        ProxyProfile {
            id: "stable-id".into(),
            name: "Primary".into(),
            protocol: ProxyProtocol::Socks5,
            host: "proxy.example.com".into(),
            port: 1080,
            authentication_enabled: false,
            credential_ref: None,
            enabled: true,
        }
    }

    #[test]
    fn editing_non_secret_fields_preserves_password_and_hides_it_from_config() {
        let store = MemoryCredentials::default();
        let mut profile = profile();
        apply_credential_update(
            &store,
            &mut profile,
            Some(CredentialUpdate::Replace {
                username: "alice".into(),
                password: "secret-value".into(),
            }),
        )
        .unwrap();
        profile.host = "changed.example.com".into();
        apply_credential_update(&store, &mut profile, None).unwrap();
        assert_eq!(
            store.get(&profile.id).unwrap().unwrap().password,
            "secret-value"
        );
        let serialized = serde_json::to_string(&profile).unwrap();
        assert!(!serialized.contains("secret-value"));
        assert!(!serialized.contains("alice"));
        assert!(profile.authentication_enabled);
    }

    #[test]
    fn disabling_authentication_deletes_stored_credentials() {
        let store = MemoryCredentials::default();
        let mut profile = profile();
        apply_credential_update(
            &store,
            &mut profile,
            Some(CredentialUpdate::Replace {
                username: "alice".into(),
                password: "secret-value".into(),
            }),
        )
        .unwrap();
        profile.authentication_enabled = false;
        apply_credential_update(&store, &mut profile, None).unwrap();
        assert!(store.get(&profile.id).unwrap().is_none());
        assert_eq!(profile.credential_ref, None);
    }

    #[test]
    fn imported_authenticated_profiles_require_new_passwords() {
        let mut configuration = PersistedConfiguration::default();
        let mut profile = profile();
        profile.authentication_enabled = true;
        profile.credential_ref = Some(profile.id.clone());
        configuration.profiles.push(profile);
        assert_eq!(
            validate_import_credentials(&configuration, &HashMap::new())
                .unwrap_err()
                .fields[0]
                .field,
            "profiles[0].credential"
        );
        let mut updates = HashMap::new();
        updates.insert(
            "stable-id".into(),
            CredentialUpdate::Replace {
                username: "alice".into(),
                password: "secret-value".into(),
            },
        );
        assert!(validate_import_credentials(&configuration, &updates).is_ok());
    }

    #[test]
    fn update_dto_does_not_serialize_credentials_or_include_them_in_errors() {
        let update: CredentialUpdate = serde_json::from_str(
            r#"{"action":"replace","username":"alice","password":"secret-value"}"#,
        )
        .unwrap();
        let store = MemoryCredentials::default();
        let mut profile = profile();
        apply_credential_update(&store, &mut profile, Some(update)).unwrap();
        let display = serde_json::to_string(&profile).unwrap();
        assert!(!display.contains("secret-value"));
        let error = apply_credential_update(
            &store,
            &mut profile,
            Some(CredentialUpdate::Replace {
                username: "alice".into(),
                password: String::new(),
            }),
        )
        .unwrap_err();
        assert!(!serde_json::to_string(&error)
            .unwrap()
            .contains("secret-value"));
    }
}
