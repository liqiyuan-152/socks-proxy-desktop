use crate::{
    error::{AppError, FieldError},
    models::{PersistedConfiguration, ProxyProfile},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const KEYRING_SERVICE: &str = "com.socks-proxy.desktop.profile";

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
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

/// Resolve only the persisted reference. Falling back to the profile ID would
/// silently use retired credentials after a versioned replacement.
pub fn read_profile_credential(
    store: &dyn CredentialStore,
    profile: &ProxyProfile,
) -> Result<Option<ProxyCredential>, AppError> {
    if !profile.authentication_enabled {
        return Ok(None);
    }
    let reference = profile
        .credential_ref
        .as_deref()
        .filter(|reference| !reference.trim().is_empty())
        .ok_or_else(credential_error)?;
    store.get(reference)
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
        context: None,
    }
}

/// Build a candidate reference and return a secret to stage after the durable
/// intent. This function never writes or deletes a keyring entry.
pub fn prepare_credential_update(
    store: &dyn CredentialStore,
    profile: &mut ProxyProfile,
    update: Option<CredentialUpdate>,
) -> Result<Option<ProxyCredential>, AppError> {
    match update.unwrap_or(CredentialUpdate::Preserve) {
        CredentialUpdate::Preserve if profile.authentication_enabled => {
            let secret = read_profile_credential(store, profile)?.ok_or_else(|| {
                AppError::validation(vec![FieldError {
                    field: "credential".into(),
                    message: "认证凭据缺失，请重新输入".into(),
                }])
            })?;
            // Copy legacy entries to a new opaque key; keep the original until
            // the new configuration reference has committed durably.
            if profile.credential_ref.as_deref() == Some(profile.id.as_str()) {
                profile.credential_ref = Some(format!("credential-v1-{}", uuid::Uuid::new_v4()));
                Ok(Some(secret))
            } else {
                Ok(None)
            }
        }
        CredentialUpdate::Replace { username, password } => {
            if username.trim().is_empty() || password.is_empty() {
                return Err(AppError::validation(vec![FieldError {
                    field: "credential".into(),
                    message: "用户名和密码不能为空".into(),
                }]));
            }
            profile.authentication_enabled = true;
            profile.credential_ref = Some(format!("credential-v1-{}", uuid::Uuid::new_v4()));
            Ok(Some(ProxyCredential { username, password }))
        }
        CredentialUpdate::Delete | CredentialUpdate::Preserve => {
            profile.authentication_enabled = false;
            profile.credential_ref = None;
            Ok(None)
        }
    }
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
        let staged = prepare_credential_update(
            &store,
            &mut profile,
            Some(CredentialUpdate::Replace {
                username: "alice".into(),
                password: "secret-value".into(),
            }),
        )
        .unwrap()
        .unwrap();
        store
            .replace(
                profile.credential_ref.as_deref().unwrap(),
                &staged.username,
                &staged.password,
            )
            .unwrap();
        profile.host = "changed.example.com".into();
        prepare_credential_update(&store, &mut profile, None).unwrap();
        assert_eq!(
            read_profile_credential(&store, &profile)
                .unwrap()
                .unwrap()
                .password,
            "secret-value"
        );
        let serialized = serde_json::to_string(&profile).unwrap();
        assert!(!serialized.contains("secret-value"));
        assert!(!serialized.contains("alice"));
        assert!(profile.authentication_enabled);
    }

    #[test]
    fn disabling_authentication_leaves_old_secret_until_commit() {
        let store = MemoryCredentials::default();
        let mut profile = profile();
        let staged = prepare_credential_update(
            &store,
            &mut profile,
            Some(CredentialUpdate::Replace {
                username: "alice".into(),
                password: "secret-value".into(),
            }),
        )
        .unwrap()
        .unwrap();
        let reference = profile.credential_ref.clone().unwrap();
        store
            .replace(&reference, &staged.username, &staged.password)
            .unwrap();
        profile.authentication_enabled = false;
        prepare_credential_update(&store, &mut profile, None).unwrap();
        assert_eq!(
            store.get(&reference).unwrap().unwrap().password,
            "secret-value"
        );
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
        prepare_credential_update(&store, &mut profile, Some(update)).unwrap();
        let display = serde_json::to_string(&profile).unwrap();
        assert!(!display.contains("secret-value"));
        let error = prepare_credential_update(
            &store,
            &mut profile,
            Some(CredentialUpdate::Replace {
                username: "alice".into(),
                password: String::new(),
            }),
        )
        .err()
        .unwrap();
        assert!(!serde_json::to_string(&error)
            .unwrap()
            .contains("secret-value"));
    }

    #[test]
    fn reads_exact_versioned_reference_and_never_falls_back_to_retired_id() {
        let store = MemoryCredentials::default();
        let mut profile = profile();
        profile.authentication_enabled = true;
        profile.credential_ref = Some("credential-v1-new".into());
        store
            .replace(&profile.id, "old-user", "old-secret")
            .unwrap();
        assert!(read_profile_credential(&store, &profile).unwrap().is_none());
        store
            .replace("credential-v1-new", "new-user", "new-secret")
            .unwrap();
        let credential = read_profile_credential(&store, &profile).unwrap().unwrap();
        assert_eq!(credential.username, "new-user");
        assert_eq!(credential.password, "new-secret");
        profile.credential_ref = Some(profile.id.clone());
        assert_eq!(
            read_profile_credential(&store, &profile)
                .unwrap()
                .unwrap()
                .password,
            "old-secret"
        );
        profile.credential_ref = None;
        assert!(read_profile_credential(&store, &profile).is_err());
        profile.authentication_enabled = false;
        assert!(read_profile_credential(&store, &profile).unwrap().is_none());
    }
}
