use crate::error::AppError;
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::Path};
use windows_sys::Win32::{
    Foundation::{CloseHandle, LocalFree, HANDLE},
    Security::{
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            SDDL_REVISION_1,
        },
        GetTokenInformation, TokenUser, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
    },
    Storage::FileSystem::CreateDirectoryW,
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

fn private_dir_error() -> AppError {
    AppError {
        code: "runtime_error".into(),
        message: "无法创建仅当前用户可访问的内核配置目录".into(),
        fields: Vec::new(),
    }
}

fn current_user_sid_string() -> Result<String, AppError> {
    let mut token: HANDLE = std::ptr::null_mut();
    // OpenProcessToken returns a handle owned by this function.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(private_dir_error());
    }
    let result = (|| {
        let mut size = 0u32;
        unsafe {
            GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut size);
        }
        if size < std::mem::size_of::<TOKEN_USER>() as u32 {
            return Err(private_dir_error());
        }
        // Use pointer-aligned storage for TOKEN_USER, which contains an SID pointer.
        let words = (size as usize).div_ceil(std::mem::size_of::<usize>());
        let mut buffer = vec![0usize; words];
        if unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                buffer.as_mut_ptr().cast(),
                size,
                &mut size,
            )
        } == 0
        {
            return Err(private_dir_error());
        }
        let sid = unsafe { (*(buffer.as_ptr() as *const TOKEN_USER)).User.Sid };
        let mut sid_text = std::ptr::null_mut();
        if sid.is_null() || unsafe { ConvertSidToStringSidW(sid, &mut sid_text) } == 0 {
            return Err(private_dir_error());
        }
        let result = (|| {
            let len = unsafe { (0..256).find(|&index| *sid_text.add(index) == 0) }
                .ok_or_else(private_dir_error)?;
            String::from_utf16(unsafe { std::slice::from_raw_parts(sid_text, len) })
                .map_err(|_| private_dir_error())
        })();
        unsafe { LocalFree(sid_text.cast()) };
        result
    })();
    unsafe { CloseHandle(token) };
    result
}

/// Creates a fresh directory with a protected DACL inherited by temporary files.
/// The path must not exist: a pre-existing directory is never trusted or reused.
pub fn create_private_runtime_dir(path: &Path) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(private_dir_error)?;
    std::fs::create_dir_all(parent).map_err(|_| private_dir_error())?;
    let sid = current_user_sid_string()?;
    let descriptor_text = format!("D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;{sid})");
    let descriptor_text: Vec<u16> = OsStr::new(&descriptor_text)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let mut descriptor = std::ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            descriptor_text.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(private_dir_error());
    }
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let created = unsafe { CreateDirectoryW(wide_path.as_ptr(), &attributes) } != 0;
    unsafe { LocalFree(descriptor) };
    if !created {
        return Err(private_dir_error());
    }
    Ok(())
}
