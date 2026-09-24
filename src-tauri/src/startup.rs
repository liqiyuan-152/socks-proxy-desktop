use crate::error::AppError;

pub trait StartupAdapter: Send + Sync {
    fn is_enabled(&self) -> Result<bool, AppError>;
    fn set_enabled(&self, enabled: bool) -> Result<(), AppError>;
}

pub struct SystemStartupAdapter;

#[cfg(windows)]
mod windows {
    use super::*;
    use std::io::ErrorKind;
    use winreg::{
        enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
        RegKey,
    };

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "Socks Proxy Desktop";

    fn expected_command() -> Result<String, AppError> {
        let executable =
            std::env::current_exe().map_err(|_| AppError::storage("无法定位应用程序"))?;
        Ok(format!("\"{}\"", executable.display()))
    }

    fn run_key() -> Result<Option<RegKey>, AppError> {
        match RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(RUN_KEY, KEY_READ | KEY_WRITE)
        {
            Ok(key) => Ok(Some(key)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(_) => Err(AppError::storage("无法访问当前用户的开机启动设置")),
        }
    }

    impl StartupAdapter for SystemStartupAdapter {
        fn is_enabled(&self) -> Result<bool, AppError> {
            let Some(key) = run_key()? else {
                return Ok(false);
            };
            match key.get_value::<String, _>(VALUE_NAME) {
                Ok(command) => Ok(command == expected_command()?),
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
                Err(_) => Err(AppError::storage("无法读取开机启动设置")),
            }
        }

        fn set_enabled(&self, enabled: bool) -> Result<(), AppError> {
            if enabled {
                let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
                    .create_subkey(RUN_KEY)
                    .map_err(|_| AppError::storage("无法写入开机启动设置"))?;
                key.set_value(VALUE_NAME, &expected_command()?)
                    .map_err(|_| AppError::storage("无法写入开机启动设置"))
            } else {
                let Some(key) = run_key()? else { return Ok(()) };
                match key.get_value::<String, _>(VALUE_NAME) {
                    Ok(command) if command == expected_command()? => key
                        .delete_value(VALUE_NAME)
                        .map_err(|_| AppError::storage("无法关闭开机启动")),
                    Ok(_) => Err(AppError::storage("开机启动项已由外部修改")),
                    Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
                    Err(_) => Err(AppError::storage("无法读取开机启动设置")),
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

    fn set_enabled(&self, enabled: bool) -> Result<(), AppError> {
        if enabled {
            Err(AppError::unavailable("当前平台不支持开机启动设置"))
        } else {
            Ok(())
        }
    }
}
