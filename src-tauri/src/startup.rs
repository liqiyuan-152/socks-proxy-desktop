use crate::{
    configuration_recovery::{StartupChange, StartupEntry},
    error::AppError,
};

pub trait StartupAdapter: Send + Sync {
    fn is_enabled(&self) -> Result<bool, AppError>;
    #[cfg(test)]
    fn set_enabled(&self, enabled: bool) -> Result<(), AppError>;

    fn read_entry(&self) -> Result<Option<StartupEntry>, AppError> {
        Err(AppError::unavailable("此启动项适配器不支持恢复"))
    }
    fn expected_entry(&self) -> Result<StartupEntry, AppError> {
        Err(AppError::unavailable("此启动项适配器不支持恢复"))
    }
    fn write_entry(&self, _: Option<&StartupEntry>) -> Result<(), AppError> {
        Err(AppError::unavailable("此启动项适配器不支持恢复"))
    }

    fn prepare_change(&self, enabled: bool) -> Result<StartupChange, AppError> {
        let original = self.read_entry()?;
        let owned = self.expected_entry()?;
        // Enabling must not replace an external value either.
        if original.as_ref().is_some_and(|entry| entry != &owned) {
            return Err(ownership_error());
        }
        Ok(StartupChange {
            original,
            expected: enabled.then_some(owned),
        })
    }

    fn compare_exchange_entry(
        &self,
        previous: Option<&StartupEntry>,
        next: Option<&StartupEntry>,
    ) -> Result<(), AppError> {
        let current = self.read_entry()?;
        if current.as_ref() == next {
            return Ok(());
        }
        if current.as_ref() != previous {
            return Err(ownership_error());
        }
        self.write_entry(next)
    }
}

fn ownership_error() -> AppError {
    AppError {
        code: "startup_ownership".into(),
        message: "开机启动项已由外部修改，请保留现场并检查设置".into(),
        fields: Vec::new(),
        context: None,
    }
}

pub struct SystemStartupAdapter;

#[cfg(windows)]
mod windows {
    use super::*;
    use std::io::ErrorKind;
    use winreg::{
        enums::{RegType, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
        types::ToRegValue,
        RegKey, RegValue,
    };

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "Socks Proxy Desktop";

    fn run_key() -> Result<Option<RegKey>, AppError> {
        match RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(RUN_KEY, KEY_READ | KEY_WRITE)
        {
            Ok(key) => Ok(Some(key)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(_) => Err(AppError::storage("无法访问当前用户的开机启动设置")),
        }
    }

    fn registry_type(value: u32) -> Result<RegType, AppError> {
        use winreg::enums::RegType::*;
        let types = [
            REG_NONE,
            REG_SZ,
            REG_EXPAND_SZ,
            REG_BINARY,
            REG_DWORD,
            REG_DWORD_BIG_ENDIAN,
            REG_LINK,
            REG_MULTI_SZ,
            REG_RESOURCE_LIST,
            REG_FULL_RESOURCE_DESCRIPTOR,
            REG_RESOURCE_REQUIREMENTS_LIST,
            REG_QWORD,
        ];
        types
            .into_iter()
            .find(|kind| kind.clone() as u32 == value)
            .ok_or_else(|| AppError::storage("启动项原值类型无效"))
    }

    impl StartupAdapter for SystemStartupAdapter {
        fn is_enabled(&self) -> Result<bool, AppError> {
            Ok(self.read_entry()?.as_ref() == Some(&self.expected_entry()?))
        }

        #[cfg(test)]
        fn set_enabled(&self, enabled: bool) -> Result<(), AppError> {
            let change = self.prepare_change(enabled)?;
            self.compare_exchange_entry(change.original.as_ref(), change.expected.as_ref())
        }

        fn read_entry(&self) -> Result<Option<StartupEntry>, AppError> {
            let Some(key) = run_key()? else {
                return Ok(None);
            };
            match key.get_raw_value(VALUE_NAME) {
                Ok(value) => Ok(Some(StartupEntry {
                    value_type: value.vtype as u32,
                    bytes: value.bytes,
                })),
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
                Err(_) => Err(AppError::storage("无法读取开机启动设置")),
            }
        }

        fn expected_entry(&self) -> Result<StartupEntry, AppError> {
            let executable =
                std::env::current_exe().map_err(|_| AppError::storage("无法定位应用程序"))?;
            let value = format!("\"{}\"", executable.display()).to_reg_value();
            Ok(StartupEntry {
                value_type: value.vtype as u32,
                bytes: value.bytes,
            })
        }

        fn write_entry(&self, entry: Option<&StartupEntry>) -> Result<(), AppError> {
            if let Some(entry) = entry {
                let value = RegValue {
                    bytes: entry.bytes.clone(),
                    vtype: registry_type(entry.value_type)?,
                };
                let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
                    .create_subkey(RUN_KEY)
                    .map_err(|_| AppError::storage("无法写入开机启动设置"))?;
                key.set_raw_value(VALUE_NAME, &value)
                    .map_err(|_| AppError::storage("无法写入开机启动设置"))
            } else {
                let Some(key) = run_key()? else { return Ok(()) };
                match key.delete_value(VALUE_NAME) {
                    Ok(()) => Ok(()),
                    Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
                    Err(_) => Err(AppError::storage("无法关闭开机启动")),
                }
            }
        }
    }
}

#[cfg(not(windows))]
impl StartupAdapter for SystemStartupAdapter {
    fn is_enabled(&self) -> Result<bool, AppError> {
        Ok(false)
    }

    #[cfg(test)]
    fn set_enabled(&self, enabled: bool) -> Result<(), AppError> {
        if enabled {
            let change = self.prepare_change(true)?;
            self.compare_exchange_entry(change.original.as_ref(), change.expected.as_ref())
        } else {
            self.compare_exchange_entry(None, None)
        }
    }

    fn read_entry(&self) -> Result<Option<StartupEntry>, AppError> {
        Ok(None)
    }

    fn expected_entry(&self) -> Result<StartupEntry, AppError> {
        Err(AppError::unavailable("当前平台不支持开机启动设置"))
    }
}

#[cfg(test)]
#[path = "startup_tests.rs"]
mod tests;
