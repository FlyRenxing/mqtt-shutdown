use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RegCloseKey, RegCreateKeyW, RegDeleteValueW, RegQueryValueExW,
    RegSetValueExW,
};

use crate::config::APP_ID;

const RUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

pub fn is_enabled() -> bool {
    match current_exe_quoted() {
        Ok(expected) => query_value().is_some_and(|value| value.starts_with(&expected)),
        Err(_) => query_value().is_some(),
    }
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    if enabled {
        write_value(&startup_command()?)
    } else {
        delete_value()
    }
}

fn startup_command() -> Result<String, String> {
    Ok(format!("{} --hidden", current_exe_quoted()?))
}

fn current_exe_quoted() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|err| format!("无法读取程序路径: {err}"))?;
    Ok(format!("\"{}\"", exe.display()))
}

fn open_run_key() -> Result<windows_sys::Win32::System::Registry::HKEY, String> {
    unsafe {
        let mut key = std::ptr::null_mut();
        let status = RegCreateKeyW(HKEY_CURRENT_USER, wide(RUN_SUBKEY).as_ptr(), &mut key);
        if status != ERROR_SUCCESS {
            return Err(format!("无法打开开机启动项 ({status})"));
        }
        Ok(key)
    }
}

fn query_value() -> Option<String> {
    unsafe {
        let key = open_run_key().ok()?;
        let name = wide(APP_ID);
        let mut kind = 0u32;
        let mut size = 0u32;
        let query = RegQueryValueExW(
            key,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            std::ptr::null_mut(),
            &mut size,
        );
        if query != ERROR_SUCCESS || size == 0 {
            RegCloseKey(key);
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        let query = RegQueryValueExW(
            key,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            buf.as_mut_ptr(),
            &mut size,
        );
        RegCloseKey(key);
        if query != ERROR_SUCCESS {
            return None;
        }
        let u16s = buf
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .take_while(|unit| *unit != 0)
            .collect::<Vec<_>>();
        Some(String::from_utf16_lossy(&u16s))
    }
}

fn write_value(command: &str) -> Result<(), String> {
    unsafe {
        let key = open_run_key()?;
        let data = wide(command);
        let bytes = std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2);
        let status = RegSetValueExW(
            key,
            wide(APP_ID).as_ptr(),
            0,
            REG_SZ,
            bytes.as_ptr(),
            bytes.len() as u32,
        );
        RegCloseKey(key);
        if status != ERROR_SUCCESS {
            return Err(format!("无法写入开机启动项 ({status})"));
        }
    }
    Ok(())
}

fn delete_value() -> Result<(), String> {
    unsafe {
        let key = open_run_key()?;
        let status = RegDeleteValueW(key, wide(APP_ID).as_ptr());
        RegCloseKey(key);
        if status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND {
            return Err(format!("无法删除开机启动项 ({status})"));
        }
    }
    Ok(())
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
