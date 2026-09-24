use crate::{
    error::AppError,
    system_proxy::{ProxySettings, ProxySettingsDevice},
};
use std::io::ErrorKind;
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
    RegKey,
};

const INTERNET_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

#[link(name = "wininet")]
extern "system" {
    fn InternetSetOptionW(
        handle: *mut std::ffi::c_void,
        option: u32,
        buffer: *mut std::ffi::c_void,
        length: u32,
    ) -> i32;
}

pub struct WindowsProxyDevice {
    key_path: String,
}

impl Default for WindowsProxyDevice {
    fn default() -> Self {
        Self {
            key_path: INTERNET_SETTINGS.into(),
        }
    }
}

fn proxy_error() -> AppError {
    AppError::unavailable("无法读取、写入或刷新当前用户的 Windows 系统代理")
}

fn read_optional<T: winreg::types::FromRegValue>(
    key: &RegKey,
    name: &str,
) -> Result<Option<T>, AppError> {
    match key.get_value(name) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(_) => Err(proxy_error()),
    }
}

fn write_optional<T: winreg::types::ToRegValue>(
    key: &RegKey,
    name: &str,
    value: &Option<T>,
) -> Result<(), AppError> {
    match value {
        Some(value) => key.set_value(name, value).map_err(|_| proxy_error()),
        None => match key.delete_value(name) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(_) => Err(proxy_error()),
        },
    }
}

impl ProxySettingsDevice for WindowsProxyDevice {
    fn read(&self) -> Result<ProxySettings, AppError> {
        let key = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(&self.key_path, KEY_READ)
            .map_err(|_| proxy_error())?;
        Ok(ProxySettings {
            enabled: read_optional(&key, "ProxyEnable")?,
            server: read_optional(&key, "ProxyServer")?,
            bypass: read_optional(&key, "ProxyOverride")?,
            auto_config_url: read_optional(&key, "AutoConfigURL")?,
            auto_detect: read_optional(&key, "AutoDetect")?,
        })
    }

    fn write(&self, settings: &ProxySettings) -> Result<(), AppError> {
        let key = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(&self.key_path, KEY_READ | KEY_WRITE)
            .map_err(|_| proxy_error())?;
        write_optional(&key, "ProxyServer", &settings.server)?;
        write_optional(&key, "ProxyOverride", &settings.bypass)?;
        write_optional(&key, "AutoConfigURL", &settings.auto_config_url)?;
        write_optional(&key, "AutoDetect", &settings.auto_detect)?;
        write_optional(&key, "ProxyEnable", &settings.enabled)?;
        // Notify other WinINet users and refresh this process after registry updates.
        let notified =
            unsafe { InternetSetOptionW(std::ptr::null_mut(), 39, std::ptr::null_mut(), 0) };
        let refreshed =
            unsafe { InternetSetOptionW(std::ptr::null_mut(), 37, std::ptr::null_mut(), 0) };
        if notified == 0 || refreshed == 0 {
            return Err(proxy_error());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        store::SqliteConfigurationStore,
        system_proxy::{OwnedSystemProxy, ProxyOwnership, ProxyOwnershipStore, SystemProxyAdapter},
    };
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Records(Arc<Mutex<Option<crate::system_proxy::ProxyOwnership>>>);

    impl ProxyOwnershipStore for Records {
        fn load_ownership(&self) -> Result<Option<crate::system_proxy::ProxyOwnership>, AppError> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn save_ownership(
            &self,
            record: &crate::system_proxy::ProxyOwnership,
        ) -> Result<(), AppError> {
            *self.0.lock().unwrap() = Some(record.clone());
            Ok(())
        }
        fn clear_ownership(&self) -> Result<(), AppError> {
            *self.0.lock().unwrap() = None;
            Ok(())
        }
    }

    #[test]
    fn registry_adapter_recovers_only_an_identified_complete_write() {
        // A unique HKCU key exercises real registry I/O without changing the
        // interactive user's Internet Settings.
        let key_path = format!(r"Software\socks-proxy-tests\{}", uuid::Uuid::new_v4());
        let root = RegKey::predef(HKEY_CURRENT_USER);
        root.create_subkey(&key_path).unwrap();
        let device = WindowsProxyDevice {
            key_path: key_path.clone(),
        };
        let original = ProxySettings {
            enabled: Some(0),
            server: Some("previous.proxy:8080".into()),
            bypass: Some("intranet".into()),
            auto_config_url: Some("http://example.invalid/proxy.pac".into()),
            auto_detect: Some(1),
        };
        device.write(&original).unwrap();
        assert_eq!(device.read().unwrap(), original);
        let records = Records::default();
        let adapter = OwnedSystemProxy::new(
            Box::new(WindowsProxyDevice {
                key_path: key_path.clone(),
            }),
            Box::new(records.clone()),
        );
        adapter.enable(18080).unwrap();
        assert_eq!(
            device.read().unwrap().server.as_deref(),
            Some("127.0.0.1:18080")
        );
        adapter.restore_if_owned().unwrap();
        assert_eq!(device.read().unwrap(), original);

        adapter.enable(18081).unwrap();
        let mut partial = device.read().unwrap();
        partial.server = Some("other.proxy:9000".into());
        device.write(&partial).unwrap();
        assert!(adapter.recover_on_startup().is_err());
        assert_eq!(device.read().unwrap(), partial);
        assert!(records.load_ownership().unwrap().is_some());
        // Cleanup only our unique test key; it is not the Internet Settings key.
        root.delete_subkey_all(&key_path).unwrap();
    }

    #[test]
    fn restart_recovers_durable_registry_ownership_and_manual_restore() {
        let key_path = format!(r"Software\socks-proxy-tests\{}", uuid::Uuid::new_v4());
        let root = RegKey::predef(HKEY_CURRENT_USER);
        root.create_subkey(&key_path).unwrap();
        let device = WindowsProxyDevice {
            key_path: key_path.clone(),
        };
        let original = ProxySettings {
            enabled: Some(0),
            server: Some("previous.proxy:8080".into()),
            bypass: Some("<local>".into()),
            auto_config_url: None,
            auto_detect: Some(1),
        };
        device.write(&original).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("configuration.sqlite3");
        {
            let adapter = OwnedSystemProxy::new(
                Box::new(WindowsProxyDevice {
                    key_path: key_path.clone(),
                }),
                Box::new(SqliteConfigurationStore::open(&database).unwrap()),
            );
            adapter.enable(18080).unwrap();
        }
        let records = SqliteConfigurationStore::open(&database).unwrap();
        assert!(records.load_ownership().unwrap().is_some());
        let restarted = OwnedSystemProxy::new(
            Box::new(WindowsProxyDevice {
                key_path: key_path.clone(),
            }),
            Box::new(SqliteConfigurationStore::open(&database).unwrap()),
        );
        restarted.recover_on_startup().unwrap();
        assert_eq!(device.read().unwrap(), original);
        assert!(records.load_ownership().unwrap().is_none());

        restarted.enable(18081).unwrap();
        restarted.restore_if_owned().unwrap();
        assert_eq!(device.read().unwrap(), original);
        assert!(records.load_ownership().unwrap().is_none());
        root.delete_subkey_all(&key_path).unwrap();
    }

    #[test]
    fn absent_auto_detect_survives_registry_takeover_and_restart_recovery() {
        let key_path = format!(r"Software\socks-proxy-tests\{}", uuid::Uuid::new_v4());
        let root = RegKey::predef(HKEY_CURRENT_USER);
        root.create_subkey(&key_path).unwrap();
        let device = WindowsProxyDevice {
            key_path: key_path.clone(),
        };
        let original = ProxySettings {
            enabled: Some(0),
            server: Some("previous.proxy:8080".into()),
            bypass: Some("<local>".into()),
            auto_config_url: None,
            auto_detect: None,
        };
        device.write(&original).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("configuration.sqlite3");
        {
            let adapter = OwnedSystemProxy::new(
                Box::new(WindowsProxyDevice {
                    key_path: key_path.clone(),
                }),
                Box::new(SqliteConfigurationStore::open(&database).unwrap()),
            );
            adapter.enable(18080).unwrap();
        }
        let records = SqliteConfigurationStore::open(&database).unwrap();
        let expected = records.load_ownership().unwrap().unwrap().expected;
        assert_eq!(expected.auto_detect, None);
        assert_eq!(device.read().unwrap(), expected);
        let restarted = OwnedSystemProxy::new(
            Box::new(WindowsProxyDevice {
                key_path: key_path.clone(),
            }),
            Box::new(SqliteConfigurationStore::open(&database).unwrap()),
        );
        restarted.recover_on_startup().unwrap();
        assert_eq!(device.read().unwrap(), original);
        assert!(records.load_ownership().unwrap().is_none());
        root.delete_subkey_all(&key_path).unwrap();
    }

    #[test]
    fn interrupted_registry_write_recovers_only_complete_owned_values() {
        let root = RegKey::predef(HKEY_CURRENT_USER);
        for state in ["original", "expected", "partial"] {
            let key_path = format!(r"Software\socks-proxy-tests\{}", uuid::Uuid::new_v4());
            root.create_subkey(&key_path).unwrap();
            let device = WindowsProxyDevice {
                key_path: key_path.clone(),
            };
            let original = ProxySettings {
                enabled: Some(0),
                server: Some("previous.proxy:8080".into()),
                bypass: Some("intranet".into()),
                auto_config_url: None,
                auto_detect: None,
            };
            let expected = ProxySettings {
                enabled: Some(1),
                server: Some("127.0.0.1:18080".into()),
                bypass: Some("<local>".into()),
                auto_config_url: None,
                auto_detect: None,
            };
            let mut partial = original.clone();
            partial.server.clone_from(&expected.server);
            device.write(&original).unwrap();
            let directory = tempfile::tempdir().unwrap();
            let database = directory.path().join("configuration.sqlite3");
            let records = SqliteConfigurationStore::open(&database).unwrap();
            let pending = ProxyOwnership {
                original: original.clone(),
                expected: original.clone(),
                pending_expected: Some(expected.clone()),
                owner_token: uuid::Uuid::new_v4().to_string(),
            };
            records.save_ownership(&pending).unwrap();
            if state != "original" {
                device
                    .write(if state == "expected" {
                        &expected
                    } else {
                        &partial
                    })
                    .unwrap();
            }
            let restarted = OwnedSystemProxy::new(
                Box::new(WindowsProxyDevice {
                    key_path: key_path.clone(),
                }),
                Box::new(SqliteConfigurationStore::open(&database).unwrap()),
            );
            if state == "partial" {
                let error = restarted.recover_on_startup().unwrap_err();
                assert!(error.message.contains("人工检查"));
                assert_eq!(device.read().unwrap(), partial);
                assert_eq!(records.load_ownership().unwrap(), Some(pending));
                assert!(restarted.enable(18081).is_err());
            } else {
                restarted.recover_on_startup().unwrap();
                assert_eq!(device.read().unwrap(), original);
                assert!(records.load_ownership().unwrap().is_none());
            }
            root.delete_subkey_all(&key_path).unwrap();
        }
    }
}
