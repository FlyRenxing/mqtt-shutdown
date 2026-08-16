use std::mem::zeroed;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{
    ERROR_ALREADY_EXISTS, FALSE, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{CreateMutexW, GetCurrentThreadId};
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    SetCurrentProcessExplicitAppUserModelID, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CallNextHookEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DestroyWindow, DispatchMessageW, EnumThreadWindows, FindWindowW, GW_OWNER, GetClassNameW,
    GetCursorPos, GetWindow, GetWindowLongPtrW, GetWindowTextW, ICON_BIG, ICON_SMALL,
    IDI_APPLICATION, IMAGE_ICON, IsWindowVisible, LR_DEFAULTSIZE, LR_LOADFROMFILE, LoadIconW,
    LoadImageW, MF_CHECKED, MF_SEPARATOR, MF_STRING, MSG, PM_REMOVE, PeekMessageW, PostMessageW,
    PostQuitMessage, RegisterClassW, SW_HIDE, SW_RESTORE, SW_SHOW, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SendMessageW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos,
    SetWindowsHookExW, ShowWindow, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RIGHTBUTTON,
    TrackPopupMenu, TranslateMessage, UnhookWindowsHookEx, WM_APP, WM_CLOSE, WM_COMMAND,
    WM_DESTROY, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NULL, WM_RBUTTONUP, WM_SETCURSOR, WM_SETICON,
    WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, HCBT_DESTROYWND, HHOOK, HTCLIENT,
};

use crate::config::{APP_ID, APP_TITLE};

const WM_TRAY: u32 = WM_APP + 1;
const WM_SHOW_MAIN: u32 = WM_APP + 2;
const ID_OPEN: usize = 1;
const ID_AUTOSTART: usize = 2;
const ID_EXIT: usize = 3;
const GWL_EXSTYLE: i32 = -20;

static ALLOW_EXIT: AtomicBool = AtomicBool::new(false);
static MAIN_HWND: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(core::ptr::null_mut());
static TRAY_HWND: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(core::ptr::null_mut());
static TRAY_ADDED: AtomicBool = AtomicBool::new(false);
static CBT_HOOK: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(core::ptr::null_mut());
static SINGLE_INSTANCE: OnceLock<isize> = OnceLock::new();
static APP_WINDOW: OnceLock<crate::app_window::AppWindowHandle> = OnceLock::new();

const APP_USER_MODEL_ID: &str = "FlyRenxing.MqttShutdown";
const IDI_APP: isize = 1;

pub fn take_hidden_flag() -> bool {
    std::env::args().any(|arg| arg == "--hidden" || arg == "/hidden")
}

pub fn set_app_user_model_id() {
    let id = wide(APP_USER_MODEL_ID);
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(id.as_ptr());
    }
}

pub fn claim_single_instance() -> bool {
    let name = wide(&format!("Local\\{APP_ID}.SingleInstance"));
    unsafe {
        let handle = CreateMutexW(core::ptr::null(), FALSE, name.as_ptr());
        if handle.is_null() {
            return true;
        }
        SINGLE_INSTANCE.get_or_init(|| handle as isize);
        if last_error() == ERROR_ALREADY_EXISTS {
            activate_existing();
            return false;
        }
    }
    true
}

pub fn attach_main_window() -> bool {
    if !MAIN_HWND.load(Ordering::SeqCst).is_null() {
        return true;
    }
    let hwnd = find_main_hwnd();
    if hwnd.is_null() {
        return false;
    }
    MAIN_HWND.store(hwnd, Ordering::SeqCst);
    apply_window_icon(hwnd);
    install_close_hook(hwnd);
    install_cbt_hook();
    create_message_window();
    add_or_update_tray();
    true
}

fn install_close_hook(hwnd: HWND) {
    let Some(app) = crate::app_window::AppWindowHandle::from_hwnd(hwnd) else {
        return;
    };
    let hooked = app.hook_close(|| {
        if ALLOW_EXIT.load(Ordering::SeqCst) {
            true
        } else {
            hide_to_tray();
            false
        }
    });
    if hooked {
        let _ = APP_WINDOW.set(app);
    }
}

fn apply_window_icon(hwnd: HWND) {
    let big = load_icon_sized(32);
    let small = load_icon_sized(16);
    unsafe {
        if !big.is_null() {
            SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, big as LPARAM);
        }
        if !small.is_null() {
            SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, small as LPARAM);
        }
    }
}

pub fn hide_to_tray() {
    let hwnd = MAIN_HWND.load(Ordering::SeqCst);
    if hwnd.is_null() {
        return;
    }
    if let Some(app) = APP_WINDOW.get() {
        app.hide();
    }
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_TOOLWINDOW as isize);
        ShowWindow(hwnd, SW_HIDE);
    }
    add_or_update_tray();
}

pub fn show_main_window() {
    let hwnd = MAIN_HWND.load(Ordering::SeqCst);
    if hwnd.is_null() {
        return;
    }
    if let Some(app) = APP_WINDOW.get() {
        app.show();
    }
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            style & !(WS_EX_TOOLWINDOW as isize),
        );
        ShowWindow(hwnd, SW_RESTORE);
        ShowWindow(hwnd, SW_SHOW);
        SetWindowPos(
            hwnd,
            core::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        SetForegroundWindow(hwnd);
        let lparam = ((WM_MOUSEMOVE << 16) | (HTCLIENT & 0xFFFF)) as LPARAM;
        let _ = PostMessageW(hwnd, WM_SETCURSOR, hwnd as WPARAM, lparam);
    }
}

pub fn request_exit() {
    ALLOW_EXIT.store(true, Ordering::SeqCst);
    remove_tray();
    crate::mqtt::stop();
    let hwnd = MAIN_HWND.load(Ordering::SeqCst);
    unsafe {
        if !hwnd.is_null() {
            let _ = PostMessageW(hwnd, WM_CLOSE, 0, 0);
        }
        let tray = TRAY_HWND.load(Ordering::SeqCst);
        if !tray.is_null() {
            DestroyWindow(tray);
        }
    }
}

fn activate_existing() {
    for _ in 0..30 {
        let tray = find_window_by_class(&message_class());
        if !tray.is_null() {
            unsafe {
                let _ = PostMessageW(tray, WM_SHOW_MAIN, 0, 0);
            }
            return;
        }
        let main = find_window_by_title(APP_TITLE);
        if !main.is_null() {
            unsafe {
                ShowWindow(main, SW_RESTORE);
                SetForegroundWindow(main);
            }
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(40));
        pump_waiting_messages();
    }
}

fn install_cbt_hook() {
    if !CBT_HOOK.load(Ordering::SeqCst).is_null() {
        return;
    }
    unsafe {
        let hook = SetWindowsHookExW(
            windows_sys::Win32::UI::WindowsAndMessaging::WH_CBT,
            Some(cbt_proc),
            core::ptr::null_mut(),
            GetCurrentThreadId(),
        );
        if !hook.is_null() {
            CBT_HOOK.store(hook, Ordering::SeqCst);
        }
    }
}

unsafe extern "system" fn cbt_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HCBT_DESTROYWND as i32 {
        let hwnd = wparam as HWND;
        if hwnd == MAIN_HWND.load(Ordering::SeqCst) && !ALLOW_EXIT.load(Ordering::SeqCst) {
            hide_to_tray();
            return 1;
        }
    }
    unsafe { CallNextHookEx(CBT_HOOK.load(Ordering::SeqCst) as HHOOK, code, wparam, lparam) }
}

fn create_message_window() {
    if !TRAY_HWND.load(Ordering::SeqCst).is_null() {
        return;
    }
    let class = wide(&message_class());
    unsafe {
        let instance = GetModuleHandleW(core::ptr::null()) as HINSTANCE;
        let wc = WNDCLASSW {
            lpfnWndProc: Some(tray_wnd_proc),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            ..zeroed()
        };
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
            class.as_ptr(),
            wide(APP_TITLE).as_ptr(),
            0,
            0,
            0,
            0,
            0,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            instance,
            core::ptr::null(),
        );
        TRAY_HWND.store(hwnd, Ordering::SeqCst);
    }
}

fn add_or_update_tray() {
    let hwnd = TRAY_HWND.load(Ordering::SeqCst);
    if hwnd.is_null() {
        return;
    }
    let mut nid = notify_data(hwnd);
    let msg = if TRAY_ADDED.load(Ordering::SeqCst) {
        NIM_MODIFY
    } else {
        NIM_ADD
    };
    unsafe {
        if Shell_NotifyIconW(msg, &nid) == 0 && msg == NIM_ADD {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &mut nid);
        }
    }
    TRAY_ADDED.store(true, Ordering::SeqCst);
}

fn remove_tray() {
    let hwnd = TRAY_HWND.load(Ordering::SeqCst);
    if hwnd.is_null() || !TRAY_ADDED.swap(false, Ordering::SeqCst) {
        return;
    }
    let nid = notify_data(hwnd);
    unsafe {
        Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

fn notify_data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid: NOTIFYICONDATAW = unsafe { zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    nid.hIcon = load_icon();
    copy_tip(&mut nid.szTip, APP_TITLE);
    nid
}

fn load_icon() -> windows_sys::Win32::UI::WindowsAndMessaging::HICON {
    load_icon_sized(0)
}

fn load_icon_sized(size: i32) -> windows_sys::Win32::UI::WindowsAndMessaging::HICON {
    let size_flags = if size == 0 { LR_DEFAULTSIZE } else { 0 };
    unsafe {
        let module = GetModuleHandleW(core::ptr::null());
        let from_exe = LoadImageW(
            module,
            IDI_APP as *const u16,
            IMAGE_ICON,
            size,
            size,
            size_flags,
        );
        if !from_exe.is_null() {
            return from_exe;
        }
    }
    if let Some(path) = crate::config::icon_path() {
        let wide_path = wide(&path.to_string_lossy());
        unsafe {
            let icon = LoadImageW(
                core::ptr::null_mut(),
                wide_path.as_ptr(),
                IMAGE_ICON,
                size,
                size,
                LR_LOADFROMFILE | size_flags,
            );
            if !icon.is_null() {
                return icon;
            }
        }
    }
    unsafe { LoadIconW(core::ptr::null_mut(), IDI_APPLICATION) }
}

fn copy_tip(dest: &mut [u16], text: &str) {
    let encoded: Vec<u16> = text.encode_utf16().collect();
    let len = encoded.len().min(dest.len().saturating_sub(1));
    dest[..len].copy_from_slice(&encoded[..len]);
    dest[len] = 0;
}

unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAY => {
            match lparam as u32 {
                WM_LBUTTONUP => show_main_window(),
                WM_RBUTTONUP => show_tray_menu(hwnd),
                _ => {}
            }
            0
        }
        WM_SHOW_MAIN => {
            show_main_window();
            0
        }
        WM_COMMAND => {
            match wparam & 0xFFFF {
                ID_OPEN => show_main_window(),
                ID_AUTOSTART => toggle_autostart(),
                ID_EXIT => request_exit(),
                _ => {}
            }
            0
        }
        WM_DESTROY => {
            remove_tray();
            if ALLOW_EXIT.load(Ordering::SeqCst) {
                unsafe { PostQuitMessage(0) };
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn toggle_autostart() {
    let next = !crate::autostart::is_enabled();
    if let Err(err) = crate::autostart::set_enabled(next) {
        message_box(&err);
    }
}

fn show_tray_menu(hwnd: HWND) {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        AppendMenuW(menu, MF_STRING, ID_OPEN, wide("打开 MQTT关机").as_ptr());
        let autostart_flags = if crate::autostart::is_enabled() {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };
        AppendMenuW(menu, autostart_flags, ID_AUTOSTART, wide("开机自启").as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, core::ptr::null());
        AppendMenuW(menu, MF_STRING, ID_EXIT, wide("退出").as_ptr());
        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        SetForegroundWindow(hwnd);
        TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            0,
            hwnd,
            core::ptr::null(),
        );
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(menu);
    }
}

fn find_main_hwnd() -> HWND {
    let mut found: HWND = core::ptr::null_mut();
    unsafe {
        let _ = EnumThreadWindows(
            GetCurrentThreadId(),
            Some(enum_main_hwnd),
            &mut found as *mut HWND as LPARAM,
        );
    }
    if !found.is_null() {
        return found;
    }
    find_window_by_title(APP_TITLE)
}

unsafe extern "system" fn enum_main_hwnd(hwnd: HWND, lparam: LPARAM) -> i32 {
    let skip = TRAY_HWND.load(Ordering::SeqCst);
    if hwnd == skip {
        return 1;
    }
    unsafe {
        if !GetWindow(hwnd, GW_OWNER).is_null() {
            return 1;
        }
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        let mut class = [0u16; 64];
        GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32);
        let class_name = String::from_utf16_lossy(&class)
            .trim_end_matches('\0')
            .to_string();
        if class_name == message_class() {
            return 1;
        }
        let mut title = [0u16; 256];
        if GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) <= 0 {
            return 1;
        }
        *(lparam as *mut HWND) = hwnd;
    }
    0
}

fn find_window_by_title(title: &str) -> HWND {
    unsafe { FindWindowW(core::ptr::null(), wide(title).as_ptr()) }
}

fn find_window_by_class(class: &str) -> HWND {
    unsafe { FindWindowW(wide(class).as_ptr(), core::ptr::null()) }
}

fn message_class() -> String {
    format!("{APP_ID}.Message")
}

fn pump_waiting_messages() {
    unsafe {
        let mut msg = zeroed::<MSG>();
        while PeekMessageW(&mut msg, core::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn last_error() -> u32 {
    unsafe { windows_sys::Win32::Foundation::GetLastError() }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn uninstall_hooks() {
    let hook = CBT_HOOK.swap(core::ptr::null_mut(), Ordering::SeqCst);
    if !hook.is_null() {
        unsafe {
            UnhookWindowsHookEx(hook as HHOOK);
        }
    }
}

pub fn message_box(text: &str) {
    let caption = wide(APP_TITLE);
    let body = wide(text);
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
            core::ptr::null_mut(),
            body.as_ptr(),
            caption.as_ptr(),
            0x10,
        );
    }
}
