use super::*;
use crate::{
    credentials::ProxyCredential,
    models::{ProxyProfile, ProxyProtocol},
    system_proxy::SystemProxyRecord,
};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    sync::atomic::{AtomicBool, Ordering},
};

struct Credentials;
impl CredentialStore for Credentials {
    fn get(&self, _: &str) -> Result<Option<ProxyCredential>, AppError> {
        Ok(None)
    }
    fn replace(&self, _: &str, _: &str, _: &str) -> Result<(), AppError> {
        Ok(())
    }
    fn delete(&self, _: &str) -> Result<(), AppError> {
        Ok(())
    }
}

#[derive(Default)]
struct ProxyState {
    port: Mutex<Option<u16>>,
    reject_next: AtomicBool,
}
impl SystemProxyAdapter for Arc<ProxyState> {
    fn enable(&self, port: u16) -> Result<SystemProxyRecord, AppError> {
        if self.reject_next.swap(false, Ordering::SeqCst) {
            return Err(AppError::unavailable("模拟系统代理切换失败"));
        }
        *self.port.lock().unwrap() = Some(port);
        Ok(SystemProxyRecord {
            original_value: "none".into(),
            expected_value: port.to_string(),
            owner_token: "test-owner".into(),
        })
    }
    fn restore_if_owned(&self) -> Result<(), AppError> {
        *self.port.lock().unwrap() = None;
        Ok(())
    }
}

fn configuration() -> PersistedConfiguration {
    let mut configuration = PersistedConfiguration::default();
    configuration.profiles.push(ProxyProfile {
        id: "stable-profile".into(),
        name: "Primary".into(),
        protocol: ProxyProtocol::Socks5,
        host: "127.0.0.1".into(),
        port: 1080,
        authentication_enabled: false,
        credential_ref: None,
        enabled: true,
    });
    configuration.active_profile_id = Some("stable-profile".into());
    configuration
}

#[test]
fn real_core_retains_old_process_until_commit_and_can_revert_candidate() {
    let Ok(binary) = std::env::var("SING_BOX_TEST_BIN") else {
        return;
    };
    let mut hasher = Sha256::new();
    std::io::copy(&mut File::open(&binary).unwrap(), &mut hasher).unwrap();
    let checksum = hex::encode(hasher.finalize());
    let directory = tempfile::tempdir().unwrap();
    let proxy = Arc::new(ProxyState::default());
    let backend = SingBoxRuntimeBackend::new(
        binary.into(),
        checksum,
        directory.path().into(),
        Arc::new(Credentials),
        Box::new(proxy.clone()),
    );
    let config = configuration();
    let first = backend
        .transition(None, &config, RuntimeMode::Global, 1)
        .unwrap()
        .unwrap();
    let first_port = *proxy.port.lock().unwrap();
    assert!(first_port.is_some());
    backend.confirm_transition(None);
    let first_secret = {
        let slots = backend.slots.lock().unwrap();
        let raw =
            std::fs::read_to_string(slots.active.as_ref().unwrap().process.config_path()).unwrap();
        serde_json::from_str::<serde_json::Value>(&raw).unwrap()["experimental"]["clash_api"]
            ["secret"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    assert_eq!(first_secret.len(), 64);
    assert!(backend.active_connections_json().unwrap()["connections"].is_array());

    proxy.reject_next.store(true, Ordering::SeqCst);
    assert!(backend
        .transition(Some(&first), &config, RuntimeMode::Rules, 2)
        .is_err());
    assert_eq!(*proxy.port.lock().unwrap(), first_port);
    assert!(backend.slots.lock().unwrap().pending.is_none());

    let second = backend
        .transition(Some(&first), &config, RuntimeMode::Rules, 2)
        .unwrap()
        .unwrap();
    assert_ne!(first.process_id, second.process_id);
    assert_eq!(second.configuration_revision, 2);
    let second_secret = {
        let slots = backend.slots.lock().unwrap();
        let raw =
            std::fs::read_to_string(slots.pending.as_ref().unwrap().process.config_path()).unwrap();
        serde_json::from_str::<serde_json::Value>(&raw).unwrap()["experimental"]["clash_api"]
            ["secret"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    assert_ne!(first_secret, second_secret);
    assert!(!format!("{second:?}").contains(&second_secret));
    assert_ne!(*proxy.port.lock().unwrap(), first_port);
    assert!(backend
        .slots
        .lock()
        .unwrap()
        .active
        .as_mut()
        .unwrap()
        .process
        .is_running()
        .unwrap());
    backend
        .revert_transition(Some(&second), Some(&first))
        .unwrap();
    assert_eq!(*proxy.port.lock().unwrap(), first_port);
    assert!(backend.slots.lock().unwrap().pending.is_none());

    let third = backend
        .transition(Some(&first), &config, RuntimeMode::Rules, 2)
        .unwrap()
        .unwrap();
    backend.confirm_transition(Some(&first));
    assert_eq!(
        backend
            .slots
            .lock()
            .unwrap()
            .active
            .as_ref()
            .unwrap()
            .identity,
        third
    );
    backend
        .transition(Some(&third), &config, RuntimeMode::Direct, 3)
        .unwrap();
    assert_eq!(*proxy.port.lock().unwrap(), None);
    backend.confirm_transition(Some(&third));
    assert!(backend.slots.lock().unwrap().active.is_none());

    let fourth = backend
        .transition(None, &config, RuntimeMode::Global, 4)
        .unwrap()
        .unwrap();
    backend.confirm_transition(None);
    backend
        .slots
        .lock()
        .unwrap()
        .active
        .as_mut()
        .unwrap()
        .process
        .stop();
    assert!(!backend.reconcile_session(&fourth).unwrap());
    assert_eq!(*proxy.port.lock().unwrap(), None);
    assert!(backend.slots.lock().unwrap().active.is_none());
    let final_session = backend
        .transition(None, &config, RuntimeMode::Global, 5)
        .unwrap()
        .unwrap();
    backend.confirm_transition(None);
    assert_eq!(
        backend
            .slots
            .lock()
            .unwrap()
            .active
            .as_ref()
            .unwrap()
            .identity,
        final_session
    );
    drop(backend);
    assert_eq!(*proxy.port.lock().unwrap(), None);
}
