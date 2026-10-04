use super::*;
use std::sync::Mutex;

#[derive(Default)]
struct Startup(Mutex<Option<StartupEntry>>);

impl StartupAdapter for Startup {
    fn is_enabled(&self) -> Result<bool, AppError> {
        Ok(self.read_entry()?.as_ref() == Some(&self.expected_entry()?))
    }
    fn set_enabled(&self, enabled: bool) -> Result<(), AppError> {
        let change = self.prepare_change(enabled)?;
        self.compare_exchange_entry(change.original.as_ref(), change.expected.as_ref())
    }
    fn read_entry(&self) -> Result<Option<StartupEntry>, AppError> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn expected_entry(&self) -> Result<StartupEntry, AppError> {
        Ok(StartupEntry {
            value_type: 1,
            bytes: vec![34, 0, 65, 0, 34, 0, 0, 0],
        })
    }
    fn write_entry(&self, entry: Option<&StartupEntry>) -> Result<(), AppError> {
        *self.0.lock().unwrap() = entry.cloned();
        Ok(())
    }
}

#[test]
fn exact_original_and_expected_survive_serialization_and_repeatable_restore() {
    let startup = Startup::default();
    let change = startup.prepare_change(true).unwrap();
    let json = serde_json::to_string(&change).unwrap();
    let change: StartupChange = serde_json::from_str(&json).unwrap();
    startup
        .compare_exchange_entry(change.original.as_ref(), change.expected.as_ref())
        .unwrap();
    assert!(startup.is_enabled().unwrap());
    startup
        .compare_exchange_entry(change.original.as_ref(), change.expected.as_ref())
        .unwrap();
    startup
        .compare_exchange_entry(change.expected.as_ref(), change.original.as_ref())
        .unwrap();
    startup
        .compare_exchange_entry(change.expected.as_ref(), change.original.as_ref())
        .unwrap();
    assert_eq!(startup.read_entry().unwrap(), None);
}

#[test]
fn external_bytes_or_type_block_apply_restore_and_both_boolean_writes() {
    for external in [
        StartupEntry {
            value_type: 1,
            bytes: vec![66, 0, 0, 0],
        },
        StartupEntry {
            value_type: 2,
            bytes: vec![34, 0, 65, 0, 34, 0, 0, 0],
        },
    ] {
        let startup = Startup::default();
        let change = startup.prepare_change(true).unwrap();
        *startup.0.lock().unwrap() = Some(external.clone());
        assert_eq!(
            startup
                .compare_exchange_entry(change.original.as_ref(), change.expected.as_ref())
                .unwrap_err()
                .code,
            "startup_ownership"
        );
        assert_eq!(
            startup
                .compare_exchange_entry(change.expected.as_ref(), change.original.as_ref())
                .unwrap_err()
                .code,
            "startup_ownership"
        );
        assert!(startup.set_enabled(true).is_err());
        assert!(startup.set_enabled(false).is_err());
        assert_eq!(startup.read_entry().unwrap(), Some(external));
    }
}

#[test]
fn disabling_owned_value_can_restore_its_exact_original() {
    let startup = Startup::default();
    startup.set_enabled(true).unwrap();
    let original = startup.read_entry().unwrap();
    let change = startup.prepare_change(false).unwrap();
    startup
        .compare_exchange_entry(change.original.as_ref(), change.expected.as_ref())
        .unwrap();
    assert_eq!(startup.read_entry().unwrap(), None);
    startup
        .compare_exchange_entry(change.expected.as_ref(), change.original.as_ref())
        .unwrap();
    assert_eq!(startup.read_entry().unwrap(), original);
}
