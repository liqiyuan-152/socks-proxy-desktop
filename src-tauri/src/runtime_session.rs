use crate::error::AppError;

pub const RUNTIME_SESSION_KEY: &str = "com.socksproxy.desktop.runtime";

pub struct SessionLease {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
    #[cfg(not(windows))]
    name: String,
}

impl SessionLease {
    pub fn acquire(name: &str) -> Result<Self, AppError> {
        if name.is_empty() {
            return Err(AppError::storage("运行时锁名称不能为空"));
        }
        acquire(name)
    }
}

fn already_owned() -> AppError {
    AppError {
        code: "runtime_already_owned".into(),
        message: "当前用户会话已有运行中的应用实例".into(),
        fields: Vec::new(),
    }
}

#[cfg(windows)]
fn acquire(name: &str) -> Result<SessionLease, AppError> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS},
        System::Threading::CreateMutexW,
    };
    let name: Vec<u16> = format!("Local\\{name}")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return Err(AppError::storage("无法获取用户会话运行时锁"));
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe { CloseHandle(handle) };
        return Err(already_owned());
    }
    Ok(SessionLease { handle })
}

#[cfg(windows)]
unsafe impl Send for SessionLease {}
#[cfg(windows)]
unsafe impl Sync for SessionLease {}

#[cfg(windows)]
impl Drop for SessionLease {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.handle) };
    }
}

#[cfg(not(windows))]
fn acquire(name: &str) -> Result<SessionLease, AppError> {
    let mut owned = local_registry()
        .lock()
        .map_err(|_| AppError::storage("运行时锁不可用"))?;
    if !owned.insert(name.to_owned()) {
        return Err(already_owned());
    }
    Ok(SessionLease {
        name: name.to_owned(),
    })
}

#[cfg(not(windows))]
fn local_registry() -> &'static std::sync::Mutex<std::collections::HashSet<String>> {
    use std::{
        collections::HashSet,
        sync::{Mutex, OnceLock},
    };
    static OWNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    OWNED.get_or_init(|| Mutex::new(HashSet::new()))
}

#[cfg(not(windows))]
impl Drop for SessionLease {
    fn drop(&mut self) {
        if let Ok(mut owned) = local_registry().lock() {
            owned.remove(&self.name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_instance_cannot_own_the_same_session() {
        let name = uuid::Uuid::new_v4().to_string();
        let first = SessionLease::acquire(&name).unwrap();
        assert_eq!(
            SessionLease::acquire(&name).err().unwrap().code,
            "runtime_already_owned"
        );
        drop(first);
        assert!(SessionLease::acquire(&name).is_ok());
    }
}
