#![allow(unsafe_op_in_unsafe_fn)]

mod install;
mod logo;
mod paint;
mod screens;
mod settings;
mod slider;
mod tray;

use std::sync::{Mutex, OnceLock};

use winapi::shared::minwindef::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use winapi::shared::windef::{HBRUSH, HWND, RECT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::wingdi::CreateSolidBrush;
use winapi::um::winuser::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, InvalidateRect,
    LoadCursorW, LoadIconW, PostQuitMessage, RegisterClassW, ReleaseCapture, SendMessageW,
    SetCapture, SetForegroundWindow, SetProcessDPIAware, SetTimer, ShowWindow, TranslateMessage,
    CW_USEDEFAULT, IDC_ARROW,
    IDI_APPLICATION, SW_HIDE, SW_SHOW, WM_CLOSE, WM_COMMAND, WM_DESTROY, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT,
    WM_RBUTTONUP, WM_SETICON, WM_TIMER, WNDCLASSW, WS_CAPTION, WS_MINIMIZEBOX, WS_OVERLAPPED,
    WS_SYSMENU,
};

use tray::{MENU_EXIT, MENU_HIDE, MENU_SHOW, WM_TRAY};

pub struct AppState {
    pub ip: String,
    pub device: String,
    pub peer: Option<String>,
    pub rtt_ms: Option<u32>,
    pub error: Option<String>,
    pub gain_percent: u32,
    pub thai: bool,
    pub screen: screens::Screen,
    pub notice: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            ip: String::new(),
            device: String::new(),
            peer: None,
            rtt_ms: None,
            error: None,
            gain_percent: 100,
            thai: true,
            screen: screens::Screen::Home,
            notice: String::new(),
        }
    }
}

pub struct Snapshot {
    pub ip: String,
    pub device: String,
    pub peer: Option<String>,
    pub rtt_ms: Option<u32>,
    pub error: Option<String>,
    pub gain_percent: u32,
    pub thai: bool,
    pub screen: screens::Screen,
    pub notice: String,
}

fn state() -> &'static Mutex<AppState> {
    static STATE: OnceLock<Mutex<AppState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(AppState::default()))
}

pub(super) fn lock() -> std::sync::MutexGuard<'static, AppState> {
    state().lock().unwrap_or_else(|err| err.into_inner())
}

pub fn set_ip(ip: String) {
    lock().ip = ip;
}

pub fn set_device(device: String) {
    lock().device = device;
}

pub fn set_peer(peer: Option<String>) {
    lock().peer = peer;
}

pub fn set_rtt(rtt_ms: Option<u32>) {
    lock().rtt_ms = rtt_ms;
}

pub fn set_error(error: String) {
    lock().error = Some(error);
}

pub fn set_gain_percent(percent: u32) {
    let percent = percent.min(200);
    lock().gain_percent = percent;
    crate::capture::set_gain_percent(percent);
    settings::save();
}

pub fn set_thai(thai: bool) {
    lock().thai = thai;
    settings::save();
}

pub fn run() {
    unsafe { run_window() }
}

pub fn alert(message: &str) {
    unsafe {
        let text = wide(message);
        let title = wide("Audrix Relay");
        winapi::um::winuser::MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), 0x10);
    }
}

pub(super) fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(super) fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | ((g as u32) << 8) | ((b as u32) << 16)
}

unsafe fn run_window() {
    settings::load();
    SetProcessDPIAware();
    let class = wide("AudrixRelay");
    let mut window_class: WNDCLASSW = std::mem::zeroed();
    window_class.lpfnWndProc = Some(wndproc);
    window_class.hInstance = GetModuleHandleW(std::ptr::null()) as HINSTANCE;
    window_class.lpszClassName = class.as_ptr();
    window_class.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
    window_class.hbrBackground = CreateSolidBrush(rgb(0x1A, 0x17, 0x14)) as HBRUSH;
    let icon = app_icon();
    window_class.hIcon = icon;
    RegisterClassW(&window_class);

    let title = wide("Audrix Relay");
    let hwnd = CreateWindowExW(
        0,
        class.as_ptr(),
        title.as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        860,
        640,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        window_class.hInstance,
        std::ptr::null_mut(),
    );
    if hwnd.is_null() {
        return;
    }
    ShowWindow(hwnd, SW_SHOW);
    SendMessageW(hwnd, WM_SETICON, winapi::um::winuser::ICON_BIG as usize, icon as isize);
    SendMessageW(hwnd, WM_SETICON, winapi::um::winuser::ICON_SMALL as usize, icon as isize);
    tray::add(hwnd, icon);
    SetTimer(hwnd, 1, 250, None);
    let mut message = std::mem::zeroed();
    while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
}

unsafe fn app_icon() -> winapi::shared::windef::HICON {
    let icon = logo::icon();
    if icon.is_null() {
        LoadIconW(std::ptr::null_mut(), IDI_APPLICATION)
    } else {
        icon
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_PAINT => {
                paint::paint(hwnd);
                0
            }
            WM_TIMER => {
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_CLOSE => {
                ShowWindow(hwnd, SW_HIDE);
                0
            }
            WM_LBUTTONDOWN => {
                on_click(hwnd, lparam, true);
                0
            }
            WM_MOUSEMOVE => {
                if (wparam & 0x0001) != 0 {
                    on_click(hwnd, lparam, false);
                }
                0
            }
            WM_LBUTTONUP => {
                let _ = ReleaseCapture();
                0
            }
            WM_TRAY => {
                match lparam as u32 {
                    WM_LBUTTONUP => tray::toggle(hwnd),
                    WM_RBUTTONUP => tray::menu(hwnd),
                    _ => {}
                }
                0
            }
            WM_COMMAND => {
                match (wparam & 0xffff) as usize {
                    MENU_SHOW => {
                        ShowWindow(hwnd, SW_SHOW);
                        SetForegroundWindow(hwnd);
                    }
                    MENU_HIDE => {
                        ShowWindow(hwnd, SW_HIDE);
                    }
                    MENU_EXIT => {
                        tray::remove(hwnd);
                        DestroyWindow(hwnd);
                    }
                    _ => {}
                }
                0
            }
            WM_DESTROY => {
                tray::remove(hwnd);
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

fn on_click(hwnd: HWND, lparam: LPARAM, first: bool) {
    let x = (lparam as i16) as i32;
    let y = ((lparam >> 16) as i16) as i32;
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    unsafe { winapi::um::winuser::GetClientRect(hwnd, &mut rect) };
    if let Some(screen) = screens::nav_at(x, y) {
        lock().screen = screen;
        unsafe { InvalidateRect(hwnd, std::ptr::null(), 0) };
        return;
    }
    let screen = lock().screen;
    let thai = lock().thai;
    if screen == screens::Screen::Settings {
        if let Some(next) = screens::lang_at(x, y) {
            set_thai(next);
            unsafe { InvalidateRect(hwnd, std::ptr::null(), 0) };
            return;
        }
        if let Some(ms) = screens::chunk_at(x, y) {
            crate::capture::set_chunk_ms(ms);
            unsafe { InvalidateRect(hwnd, std::ptr::null(), 0) };
            return;
        }
        let bar = screens::volume_bar();
        if slider::contains(bar, x, y) {
            if first {
                unsafe { SetCapture(hwnd) };
            }
            set_gain_percent(slider::percent_at(bar, x));
            unsafe { InvalidateRect(hwnd, std::ptr::null(), 0) };
        }
    }
    if screen == screens::Screen::About {
        if screens::install_at(x, y) {
            let notice = match install::install() {
                Ok(path) => if thai {
                    format!("ติดตั้งแล้วที่ {path}")
                } else {
                    format!("Installed at {path}")
                },
                Err(err) => err,
            };
            lock().notice = notice;
            unsafe { InvalidateRect(hwnd, std::ptr::null(), 0) };
        } else if screens::uninstall_at(x, y) {
            let notice = match install::uninstall() {
                Ok(()) => {
                    if thai { "ถอนการติดตั้งแล้ว".into() } else { "Removed from the Start menu.".into() }
                }
                Err(err) => err,
            };
            lock().notice = notice;
            unsafe { InvalidateRect(hwnd, std::ptr::null(), 0) };
        }
    }
}
