//! Settings and the "Start with Windows" entry, both kept under HKEY_CURRENT_USER.

use std::ffi::c_void;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_DWORD, REG_SZ, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW,
    RegSetKeyValueW,
};
use windows_sys::core::PCWSTR;
use windows_sys::w;

const SETTINGS_KEY: PCWSTR = w!("Software\\green-state");
const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const RUN_VALUE: PCWSTR = w!("green-state");

pub fn get_setting(name: PCWSTR) -> Option<u32> {
    let mut value = 0u32;
    let mut len = size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            SETTINGS_KEY,
            name,
            RRF_RT_REG_DWORD,
            null_mut(),
            &mut value as *mut u32 as *mut c_void,
            &mut len,
        )
    };
    (status == ERROR_SUCCESS).then_some(value)
}

/// Creates the settings key on first use.
pub fn set_setting(name: PCWSTR, value: u32) {
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            SETTINGS_KEY,
            name,
            REG_DWORD,
            &value as *const u32 as *const c_void,
            size_of::<u32>() as u32,
        )
    };
}

pub fn starts_with_windows() -> bool {
    // With no buffer the call only reports the value's size, which succeeds iff the value exists.
    let mut len = 0u32;
    let status = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_VALUE, RRF_RT_REG_SZ, null_mut(), null_mut(), &mut len)
    };
    status == ERROR_SUCCESS
}

/// Registers the running executable (quoted, as paths may contain spaces) to start at sign-in.
pub fn set_start_with_windows(enabled: bool) {
    if !enabled {
        unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_VALUE) };
        return;
    }
    let Ok(exe) = std::env::current_exe() else { return };
    let command: Vec<u16> =
        "\"".encode_utf16().chain(exe.as_os_str().encode_wide()).chain("\"\0".encode_utf16()).collect();
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            RUN_VALUE,
            REG_SZ,
            command.as_ptr() as *const c_void,
            (command.len() * size_of::<u16>()) as u32,
        )
    };
}
