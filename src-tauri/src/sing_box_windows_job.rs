use crate::error::AppError;
use std::{os::windows::io::AsRawHandle, process::Child};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::JobObjects::{
        JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    },
    System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
};

/// Closing the last handle kills the assigned core, including when the desktop
/// process is force-terminated and Rust destructors are not run.
pub struct CoreJob(HANDLE);

impl CoreJob {
    pub fn new() -> Result<Self, AppError> {
        type CreateJob = unsafe extern "system" fn(*const std::ffi::c_void, *const u16) -> HANDLE;
        let create: CreateJob = unsafe { std::mem::transmute(resolve(b"CreateJobObjectW\0")?) };
        let handle = unsafe { create(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(job_error());
        }
        let job = Self(handle);
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        type SetJob = unsafe extern "system" fn(HANDLE, i32, *const std::ffi::c_void, u32) -> i32;
        let set: SetJob = unsafe { std::mem::transmute(resolve(b"SetInformationJobObject\0")?) };
        if unsafe {
            set(
                job.0,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        } == 0
        {
            return Err(job_error());
        }
        Ok(job)
    }

    pub fn attach(&self, child: &Child) -> Result<(), AppError> {
        type AssignJob = unsafe extern "system" fn(HANDLE, HANDLE) -> i32;
        let assign: AssignJob =
            unsafe { std::mem::transmute(resolve(b"AssignProcessToJobObject\0")?) };
        let process = child.as_raw_handle() as HANDLE;
        if unsafe { assign(self.0, process) } == 0 {
            return Err(job_error());
        }
        Ok(())
    }
}

fn resolve(name: &[u8]) -> Result<unsafe extern "system" fn() -> isize, AppError> {
    let kernel32: Vec<u16> = "kernel32.dll\0".encode_utf16().collect();
    let module = unsafe { GetModuleHandleW(kernel32.as_ptr()) };
    if module.is_null() {
        return Err(job_error());
    }
    unsafe { GetProcAddress(module, name.as_ptr()) }.ok_or_else(job_error)
}

impl Drop for CoreJob {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

// A job handle is only used to assign the child during startup and then owned
// until shutdown; moving its handle between threads does not change ownership.
unsafe impl Send for CoreJob {}

fn job_error() -> AppError {
    AppError {
        code: "runtime_error".into(),
        message: "无法保护代理内核的异常退出清理".into(),
        fields: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        process::Command,
        time::{Duration, Instant},
    };

    #[test]
    fn closing_job_kills_only_assigned_child() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/fake-sing-box.rs");
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("fake-sing-box.exe");
        assert!(Command::new("rustc")
            .arg("--edition=2021")
            .arg(fixture)
            .arg("-o")
            .arg(&binary)
            .status()
            .unwrap()
            .success());
        let mut owned = Command::new(&binary).arg("idle").spawn().unwrap();
        let mut unrelated = Command::new(&binary).arg("idle").spawn().unwrap();
        let job = CoreJob::new().unwrap();
        job.attach(&owned).unwrap();
        drop(job);
        let deadline = Instant::now() + Duration::from_secs(3);
        while owned.try_wait().unwrap().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(owned.try_wait().unwrap().is_some());
        assert!(unrelated.try_wait().unwrap().is_none());
        unrelated.kill().unwrap();
        unrelated.wait().unwrap();
    }
}
