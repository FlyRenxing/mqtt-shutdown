#![windows_subsystem = "windows"]

use std::env;
use std::fs;
use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/payload.tar"));

fn main() {
    if PAYLOAD.is_empty() {
        fatal("打包器缺少 payload，请先运行 tools/pack.ps1。");
    }
    let dest = runtime_dir();
    let stamp = dest.join(".payload-id");
    let id = payload_id();
    let ready = fs::read_to_string(&stamp).ok().is_some_and(|s| s == id);
    if !ready {
        if let Err(err) = extract_tar(PAYLOAD, &dest) {
            fatal(&format!("无法解压运行时: {err}"));
        }
        let _ = fs::write(&stamp, id);
    }
    let exe = dest.join("mqtt-shutdown.exe");
    if !exe.is_file() {
        fatal(&format!("找不到 {}", exe.display()));
    }
    let mut cmd = Command::new(&exe);
    cmd.args(env::args().skip(1))
        .current_dir(&dest)
        .creation_flags(CREATE_NO_WINDOW);
    if let Err(err) = cmd.spawn() {
        fatal(&format!("无法启动: {err}"));
    }
}

fn runtime_dir() -> PathBuf {
    let base = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::temp_dir());
    base.join("MqttShutdown")
        .join("runtime")
        .join(env!("CARGO_PKG_VERSION"))
}

fn payload_id() -> String {
    let mut hash = 0x811c9dc5_u32;
    for byte in PAYLOAD {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    format!("{:08x}-{}", hash, PAYLOAD.len())
}

fn extract_tar(bytes: &[u8], dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    let mut offset = 0usize;
    while offset + 512 <= bytes.len() {
        let header = &bytes[offset..offset + 512];
        if header.iter().all(|b| *b == 0) {
            break;
        }
        let name = tar_name(header);
        let size = tar_octal(&header[124..136]);
        let kind = header[156];
        offset += 512;
        if offset + size > bytes.len() {
            break;
        }
        let data = &bytes[offset..offset + size];
        offset += size.div_ceil(512) * 512;
        if name.is_empty() || name.contains("..") {
            continue;
        }
        let path = dest.join(name.replace('/', "\\"));
        match kind {
            b'5' => fs::create_dir_all(path)?,
            b'0' | 0 => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(path, data)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn tar_name(header: &[u8]) -> String {
    let prefix = cstr(&header[345..500]);
    let name = cstr(&header[0..100]);
    if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    }
}

fn cstr(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

fn tar_octal(bytes: &[u8]) -> usize {
    usize::from_str_radix(cstr(bytes).trim(), 8).unwrap_or(0)
}

fn fatal(text: &str) -> ! {
    let body: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let title: Vec<u16> = "MQTT关机".encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        MessageBoxW(
            core::ptr::null_mut(),
            body.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
    std::process::exit(1);
}
