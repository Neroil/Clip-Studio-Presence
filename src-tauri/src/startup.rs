#[cfg(windows)]
use std::{env, ffi::OsStr, os::windows::ffi::OsStrExt, ptr::null_mut};

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
    System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
        KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    },
};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE_NAME: &str = "Clip Studio Presence";

pub fn set_start_on_boot(enabled: bool) -> Result<(), StartupError> {
    set_start_on_boot_inner(enabled)
}

#[cfg(windows)]
fn set_start_on_boot_inner(enabled: bool) -> Result<(), StartupError> {
    let mut key: HKEY = null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide_null(RUN_KEY).as_ptr(),
            0,
            null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            null_mut(),
            &mut key,
            null_mut(),
        )
    };

    if status != ERROR_SUCCESS {
        return Err(StartupError::Registry(status));
    }

    let result = if enabled {
        let exe = env::current_exe()?;
        let command = format!("\"{}\"", exe.display());
        let command_wide = wide_null(&command);
        let bytes = command_wide.len() * std::mem::size_of::<u16>();
        let status = unsafe {
            RegSetValueExW(
                key,
                wide_null(RUN_VALUE_NAME).as_ptr(),
                0,
                REG_SZ,
                command_wide.as_ptr().cast::<u8>(),
                bytes as u32,
            )
        };

        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(StartupError::Registry(status))
        }
    } else {
        let status = unsafe { RegDeleteValueW(key, wide_null(RUN_VALUE_NAME).as_ptr()) };
        if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(StartupError::Registry(status))
        }
    };

    unsafe {
        RegCloseKey(key);
    }

    result
}

#[cfg(not(windows))]
fn set_start_on_boot_inner(_enabled: bool) -> Result<(), StartupError> {
    Err(StartupError::UnsupportedPlatform)
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("could not resolve the app executable path: {0}")]
    CurrentExe(#[from] std::io::Error),
    #[cfg(windows)]
    #[error("Windows startup registry update failed with status code {0}")]
    Registry(u32),
    #[cfg(not(windows))]
    #[error("start on boot is currently only implemented on Windows")]
    UnsupportedPlatform,
}
