use super::*;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct MemoryDevice {
    value: Arc<Mutex<ProxySettings>>,
    fail_next_write: Arc<Mutex<bool>>,
    fail_after_partial_write: Arc<Mutex<bool>>,
}

impl ProxySettingsDevice for MemoryDevice {
    fn read(&self) -> Result<ProxySettings, AppError> {
        Ok(self.value.lock().unwrap().clone())
    }

    fn write(&self, settings: &ProxySettings) -> Result<(), AppError> {
        if std::mem::take(&mut *self.fail_next_write.lock().unwrap()) {
            return Err(AppError::unavailable("模拟写入失败"));
        }
        if std::mem::take(&mut *self.fail_after_partial_write.lock().unwrap()) {
            self.value
                .lock()
                .unwrap()
                .server
                .clone_from(&settings.server);
            return Err(AppError::unavailable("模拟逐字段写入中断"));
        }
        *self.value.lock().unwrap() = settings.clone();
        Ok(())
    }
}

#[derive(Clone, Default)]
struct MemoryRecords(Arc<Mutex<Option<ProxyOwnership>>>);

impl ProxyOwnershipStore for MemoryRecords {
    fn load_ownership(&self) -> Result<Option<ProxyOwnership>, AppError> {
        Ok(self.0.lock().unwrap().clone())
    }

    fn save_ownership(&self, record: &ProxyOwnership) -> Result<(), AppError> {
        *self.0.lock().unwrap() = Some(record.clone());
        Ok(())
    }

    fn clear_ownership(&self) -> Result<(), AppError> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}

fn fixture() -> (OwnedSystemProxy, MemoryDevice, MemoryRecords, ProxySettings) {
    let original = ProxySettings {
        enabled: Some(1),
        server: Some("other.proxy:8888".into()),
        bypass: Some("intranet".into()),
        auto_config_url: Some("https://example.org/config.pac".into()),
        auto_detect: Some(1),
    };
    let device = MemoryDevice {
        value: Arc::new(Mutex::new(original.clone())),
        fail_next_write: Arc::new(Mutex::new(false)),
        fail_after_partial_write: Arc::new(Mutex::new(false)),
    };
    let records = MemoryRecords::default();
    let adapter = OwnedSystemProxy::new(Box::new(device.clone()), Box::new(records.clone()));
    (adapter, device, records, original)
}

#[test]
fn applying_switching_and_restoring_preserves_all_original_values() {
    let (adapter, device, records, original) = fixture();
    let first = adapter.enable(18080).unwrap();
    assert_eq!(
        device.read().unwrap().server.as_deref(),
        Some("127.0.0.1:18080")
    );
    assert_ne!(first.owner_token, "");
    assert_eq!(
        serde_json::from_str::<ProxySettings>(&first.original_value).unwrap(),
        original
    );
    assert_eq!(
        serde_json::from_str::<ProxySettings>(&first.expected_value).unwrap(),
        device.read().unwrap()
    );
    assert!(device.read().unwrap().auto_config_url.is_none());
    let second = adapter.enable(18081).unwrap();
    assert_eq!(first.owner_token, second.owner_token);
    assert_eq!(
        records.load_ownership().unwrap().unwrap().original,
        original
    );
    adapter.restore_if_owned().unwrap();
    assert_eq!(device.read().unwrap(), original);
    assert!(records.load_ownership().unwrap().is_none());
}

#[test]
fn a_new_instance_restores_abandoned_or_pending_owned_settings() {
    let (adapter, device, records, original) = fixture();
    adapter.enable(18080).unwrap();
    let restarted = OwnedSystemProxy::new(Box::new(device.clone()), Box::new(records.clone()));
    restarted.recover_on_startup().unwrap();
    assert_eq!(device.read().unwrap(), original);
    assert!(records.load_ownership().unwrap().is_none());

    let mut pending = original.clone();
    pending.enabled = Some(1);
    pending.server = Some("127.0.0.1:18082".into());
    records
        .save_ownership(&ProxyOwnership {
            original: original.clone(),
            expected: original.clone(),
            pending_expected: Some(pending.clone()),
            owner_token: "abandoned".into(),
        })
        .unwrap();
    device.write(&pending).unwrap();
    restarted.recover_on_startup().unwrap();
    assert_eq!(device.read().unwrap(), original);
}

#[test]
fn external_change_is_never_overwritten_and_keeps_recovery_evidence() {
    let (adapter, device, records, original) = fixture();
    adapter.enable(18080).unwrap();
    let mut external = original.clone();
    external.server = Some("external.proxy:9000".into());
    device.write(&external).unwrap();
    assert_eq!(adapter.restore_if_owned().unwrap_err().code, "unavailable");
    assert_eq!(device.read().unwrap(), external);
    assert!(records.load_ownership().unwrap().is_some());
    assert!(adapter.enable(18081).is_err());
}

#[test]
fn pending_write_with_unchanged_original_clears_journal_without_writing() {
    let (adapter, device, records, original) = fixture();
    let mut expected = original.clone();
    expected.server = Some("127.0.0.1:18080".into());
    records
        .save_ownership(&ProxyOwnership {
            original: original.clone(),
            expected: original.clone(),
            pending_expected: Some(expected),
            owner_token: "pending".into(),
        })
        .unwrap();
    adapter.recover_on_startup().unwrap();
    assert_eq!(device.read().unwrap(), original);
    assert!(records.load_ownership().unwrap().is_none());
}

#[test]
fn interrupted_partial_write_is_never_overwritten_even_after_restart() {
    let (adapter, device, records, original) = fixture();
    let mut expected = original.clone();
    expected.enabled = Some(1);
    expected.server = Some("127.0.0.1:18080".into());
    expected.bypass = Some("<local>".into());
    let pending = ProxyOwnership {
        original: original.clone(),
        expected: original.clone(),
        pending_expected: Some(expected),
        owner_token: "pending".into(),
    };
    records.save_ownership(&pending).unwrap();
    let mut partial = original;
    partial.server = Some("127.0.0.1:18080".into());
    device.write(&partial).unwrap();
    let restarted = OwnedSystemProxy::new(Box::new(device.clone()), Box::new(records.clone()));
    assert!(restarted
        .recover_on_startup()
        .unwrap_err()
        .message
        .contains("人工检查"));
    assert!(restarted.restore_if_owned().is_err());
    assert!(adapter.enable(18081).is_err());
    assert_eq!(device.read().unwrap(), partial);
    assert_eq!(records.load_ownership().unwrap(), Some(pending));
}

#[test]
fn failing_during_partial_write_keeps_pending_journal_and_blocks_takeover() {
    let (adapter, device, records, original) = fixture();
    *device.fail_after_partial_write.lock().unwrap() = true;
    let failure = adapter.enable(18080).err().unwrap();
    assert!(failure.message.contains("人工检查"));
    let partial = device.read().unwrap();
    assert_eq!(partial.server.as_deref(), Some("127.0.0.1:18080"));
    assert_eq!(partial.bypass, original.bypass);
    let record = records.load_ownership().unwrap().unwrap();
    assert_eq!(record.original, original);
    assert!(record.pending_expected.is_some());
    assert_eq!(adapter.enable(18081).err().unwrap().code, "unavailable");
    assert!(adapter.restore_if_owned().is_err());
    assert_eq!(device.read().unwrap(), partial);
    assert_eq!(records.load_ownership().unwrap(), Some(record));
}

#[test]
fn failed_write_rolls_back_journal_without_losing_previous_session() {
    let (adapter, device, records, _) = fixture();
    adapter.enable(18080).unwrap();
    let previous = records.load_ownership().unwrap();
    *device.fail_next_write.lock().unwrap() = true;
    assert!(matches!(adapter.enable(18081), Err(AppError { code, .. }) if code == "unavailable"));
    assert_eq!(
        device.read().unwrap().server.as_deref(),
        Some("127.0.0.1:18080")
    );
    assert_eq!(records.load_ownership().unwrap(), previous);
}

#[test]
fn enabling_proxy_preserves_absent_auto_detect_in_expected_journal() {
    let (adapter, device, records, mut original) = fixture();
    original.auto_detect = None;
    device.write(&original).unwrap();
    adapter.enable(18080).unwrap();
    let recorded = records.load_ownership().unwrap().unwrap();
    assert_eq!(recorded.expected.auto_detect, None);
    assert_eq!(device.read().unwrap().auto_detect, None);
    adapter.restore_if_owned().unwrap();
    assert_eq!(device.read().unwrap(), original);
}
