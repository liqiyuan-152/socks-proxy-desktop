use crate::{
    configuration_recovery::StartupEntry,
    credentials::{CredentialStore, ProxyCredential},
    error::AppError,
    startup::StartupAdapter,
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

// Synthetic secrets are persisted in a separate test keyring, never in the
// application's SQLite journal. Each write is durable before its checkpoint.
pub(super) struct Credentials(pub PathBuf);
impl CredentialStore for Credentials {
    fn get(&self, reference: &str) -> Result<Option<ProxyCredential>, AppError> {
        let path = self.0.join(reference);
        if !path.exists() {
            return Ok(None);
        }
        let values: (String, String) = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        Ok(Some(ProxyCredential {
            username: values.0,
            password: values.1,
        }))
    }
    fn replace(&self, reference: &str, username: &str, password: &str) -> Result<(), AppError> {
        fs::create_dir_all(&self.0).unwrap();
        persist(
            &self.0.join(reference),
            &serde_json::to_vec(&(username, password)).unwrap(),
        );
        Ok(())
    }
    fn delete(&self, reference: &str) -> Result<(), AppError> {
        match fs::remove_file(self.0.join(reference)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(AppError::storage("fixture keyring delete")),
        }
    }
}

pub(super) struct Startup(pub PathBuf);
impl StartupAdapter for Startup {
    fn is_enabled(&self) -> Result<bool, AppError> {
        Ok(self.read_entry()?.as_ref() == Some(&self.expected_entry()?))
    }
    fn set_enabled(&self, _: bool) -> Result<(), AppError> {
        panic!("boolean write bypass");
    }
    fn read_entry(&self) -> Result<Option<StartupEntry>, AppError> {
        if !self.0.exists() {
            return Ok(None);
        }
        Ok(Some(
            serde_json::from_slice(&fs::read(&self.0).unwrap()).unwrap(),
        ))
    }
    fn expected_entry(&self) -> Result<StartupEntry, AppError> {
        Ok(StartupEntry {
            value_type: 1,
            bytes: vec![65, 0, 0, 0],
        })
    }
    fn write_entry(&self, value: Option<&StartupEntry>) -> Result<(), AppError> {
        if let Some(value) = value {
            persist(&self.0, &serde_json::to_vec(value).unwrap());
        } else if self.0.exists() {
            fs::remove_file(&self.0).unwrap();
        }
        Ok(())
    }
}

pub(super) fn persist(path: &Path, bytes: &[u8]) {
    let mut file = fs::File::create(path).unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
