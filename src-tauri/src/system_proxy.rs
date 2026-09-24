use crate::error::AppError;
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProxySettings {
    pub enabled: Option<u32>,
    pub server: Option<String>,
    pub bypass: Option<String>,
    pub auto_config_url: Option<String>,
    pub auto_detect: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProxyOwnership {
    pub original: ProxySettings,
    pub expected: ProxySettings,
    pub pending_expected: Option<ProxySettings>,
    pub owner_token: String,
}

pub trait ProxyOwnershipStore: Send + Sync {
    fn load_ownership(&self) -> Result<Option<ProxyOwnership>, AppError>;
    fn save_ownership(&self, record: &ProxyOwnership) -> Result<(), AppError>;
    fn clear_ownership(&self) -> Result<(), AppError>;
}

pub trait ProxySettingsDevice: Send + Sync {
    fn read(&self) -> Result<ProxySettings, AppError>;
    fn write(&self, settings: &ProxySettings) -> Result<(), AppError>;
}

/// Owns only the settings it can still identify as its own. All callers must
/// hold the per-user session lease before constructing or using this adapter.
pub struct OwnedSystemProxy {
    device: Box<dyn ProxySettingsDevice>,
    records: Box<dyn ProxyOwnershipStore>,
}

impl OwnedSystemProxy {
    pub fn new(
        device: Box<dyn ProxySettingsDevice>,
        records: Box<dyn ProxyOwnershipStore>,
    ) -> Self {
        Self { device, records }
    }

    /// Called before accepting any new runtime transition after a restart.
    pub fn recover_on_startup(&self) -> Result<(), AppError> {
        self.restore_if_owned()
    }

    fn restore_record(&self, record: ProxyOwnership) -> Result<(), AppError> {
        let current = self.device.read()?;
        if current == record.original {
            return self.records.clear_ownership();
        }
        if current != record.expected && record.pending_expected.as_ref() != Some(&current) {
            return Err(AppError::unavailable(
                "系统代理当前值无法确认归属；保留当前设置及恢复记录，请人工检查",
            ));
        }
        self.device.write(&record.original)?;
        self.records.clear_ownership()
    }

    fn rollback_enable(
        &self,
        before: Option<&ProxyOwnership>,
        previous: &ProxySettings,
        attempted: &ProxySettings,
    ) -> Result<(), AppError> {
        let current = self.device.read()?;
        if current == *attempted {
            self.device.write(previous)?;
        } else if current != *previous {
            return Err(AppError::unavailable(
                "系统代理写入未完成且当前值无法确认；保留当前设置及恢复记录，请人工检查",
            ));
        }
        if let Some(record) = before {
            self.records.save_ownership(record)
        } else {
            self.records.clear_ownership()
        }
    }
}

pub struct SystemProxyRecord {
    pub original_value: String,
    pub expected_value: String,
    pub owner_token: String,
}

pub trait SystemProxyAdapter: Send + Sync {
    fn enable(&self, localhost_port: u16) -> Result<SystemProxyRecord, AppError>;
    fn restore_if_owned(&self) -> Result<(), AppError>;
}

impl SystemProxyAdapter for OwnedSystemProxy {
    fn enable(&self, localhost_port: u16) -> Result<SystemProxyRecord, AppError> {
        if localhost_port == 0 {
            return Err(AppError::unavailable("系统代理监听端口无效"));
        }
        let current = self.device.read()?;
        let existing = self.records.load_ownership()?;
        if let Some(record) = &existing {
            if record.pending_expected.is_some() || current != record.expected {
                return Err(AppError::unavailable(
                    "系统代理所有权不确定，需先恢复网络设置",
                ));
            }
        }
        let mut expected = current.clone();
        expected.enabled = Some(1);
        expected.server = Some(format!("127.0.0.1:{localhost_port}"));
        expected.bypass = Some("<local>".into());
        expected.auto_config_url = None;
        // An absent AutoDetect value already means no automatic discovery.
        // WinINet may remove a newly written zero during refresh, so do not
        // introduce that value when it was absent before this transition.
        expected.auto_detect = current.auto_detect.map(|_| 0);
        let original = existing
            .as_ref()
            .map_or_else(|| current.clone(), |r| r.original.clone());
        let owner_token = existing.as_ref().map_or_else(
            || {
                let mut bytes = [0u8; 32];
                rand::rngs::OsRng.fill_bytes(&mut bytes);
                hex::encode(bytes)
            },
            |record| record.owner_token.clone(),
        );
        let record = ProxyOwnership {
            original: original.clone(),
            expected: current.clone(),
            pending_expected: Some(expected.clone()),
            owner_token: owner_token.clone(),
        };
        self.records.save_ownership(&record)?;
        // If the platform write or final journal update fails, the durable
        // pending value enables recovery without guessing who owns the proxy.
        let result = self.device.write(&expected).and_then(|()| {
            self.records.save_ownership(&ProxyOwnership {
                expected: expected.clone(),
                pending_expected: None,
                ..record
            })
        });
        if let Err(error) = result {
            self.rollback_enable(existing.as_ref(), &current, &expected)?;
            return Err(error);
        }
        Ok(SystemProxyRecord {
            original_value: serde_json::to_string(&original).unwrap_or_default(),
            expected_value: serde_json::to_string(&expected).unwrap_or_default(),
            owner_token,
        })
    }

    fn restore_if_owned(&self) -> Result<(), AppError> {
        if let Some(record) = self.records.load_ownership()? {
            self.restore_record(record)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "system_proxy_tests.rs"]
mod tests;
