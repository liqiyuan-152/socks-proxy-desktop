use socks_proxy_lib::services::{
    adapters::{CredentialStore, ProxyCredential},
    AppError,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone, Default)]
pub struct InMemoryCredentialStore(Arc<Mutex<HashMap<String, (String, String)>>>);
impl CredentialStore for InMemoryCredentialStore {
    fn get(&self, id: &str) -> Result<Option<ProxyCredential>, AppError> {
        let values = self
            .0
            .lock()
            .map_err(|_| AppError::storage("测试凭据锁不可用"))?;
        Ok(values.get(id).map(|(username, password)| ProxyCredential {
            username: username.clone(),
            password: password.clone(),
        }))
    }
    fn replace(&self, id: &str, username: &str, password: &str) -> Result<(), AppError> {
        self.0
            .lock()
            .map_err(|_| AppError::storage("测试凭据锁不可用"))?
            .insert(id.into(), (username.into(), password.into()));
        Ok(())
    }
    fn delete(&self, id: &str) -> Result<(), AppError> {
        self.0
            .lock()
            .map_err(|_| AppError::storage("测试凭据锁不可用"))?
            .remove(id);
        Ok(())
    }
}
