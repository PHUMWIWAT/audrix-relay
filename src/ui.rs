#![allow(unsafe_op_in_unsafe_fn)]

use std::sync::{Mutex, OnceLock};

use winapi::shared::minwindef::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use winapi::shared::windef::{HBRUSH, HDC, HWND, RECT, SIZE};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::wingdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush, DeleteDC,
    DeleteObject, GetTextExtentPoint32W, SelectObject, SetBkMode, SetTextColor, CLR_INVALID, DEFAULT_CHARSET,
    DEFAULT_PITCH, FF_DONTCARE, FW_NORMAL, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
    CLEARTYPE_QUALITY, SRCCOPY, TRANSPARENT,
};
use winapi::um::winuser::{
    BeginPaint, CreateWindowExW, DefWindowProcW, DispatchMessageW, DrawTextW, EndPaint, FillRect,
    GetClientRect, GetMessageW, InvalidateRect, LoadCursorW, PostQuitMessage, RegisterClassW,
    SetProcessDPIAware, SetTimer, ShowWindow, TranslateMessage, CW_USEDEFAULT,
    DT_LEFT, DT_SINGLELINE, IDC_ARROW, SW_SHOW, WM_DESTROY, WM_PAINT, WM_TIMER, WNDCLASSW,
    WS_CAPTION, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU,
};

pub struct AppState {
    pub ip: String,
    pub device: String,
    pub peer: Option<String>,
    pub rtt_ms: Option<u32>,
    pub error: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            ip: String::new(),
            device: String::new(),
            peer: None,
            rtt_ms: None,
            error: None,
        }
    }
}

fn state() -> &'static Mutex<AppState> {
    static STATE: OnceLock<Mutex<AppState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(AppState::default()))
}

fn lock() -> std::sync::MutexGuard<'static, AppState> {
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

pub fn run() {
    unsafe { run_window() }
}

unsafe fn run_window() {
    SetProcessDPIAware();
    let class = wide("AudrixRelay");
    let mut window_class: WNDCLASSW = std::mem::zeroed();
    window_class.lpfnWndProc = Some(wndproc);
    window_class.hInstance = GetModuleHandleW(std::ptr::null()) as HINSTANCE;
    window_class.lpszClassName = class.as_ptr();
    window_class.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
    window_class.hbrBackground = CreateSolidBrush(rgb(0x1A, 0x17, 0x14)) as HBRUSH;
    RegisterClassW(&window_class);

    let title = wide("Audrix Relay");
    let hwnd = CreateWindowExW(
        0,
        class.as_ptr(),
        title.as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        440,
        560,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        window_class.hInstance,
        std::ptr::null_mut(),
    );
    if hwnd.is_null() {
        return;
    }
    ShowWindow(hwnd, SW_SHOW);
    SetTimer(hwnd, 1, 250, None);
    let mut message = std::mem::zeroed();
    while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_PAINT => {
                paint(hwnd);
                0
            }
            WM_TIMER => {
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

unsafe fn paint(hwnd: HWND) {
    let mut paint_struct = std::mem::zeroed();
    let hdc = BeginPaint(hwnd, &mut paint_struct);
    let mut rect = std::mem::zeroed();
    GetClientRect(hwnd, &mut rect);
    let width = rect.right;
    let height = rect.bottom;
    let mem = CreateCompatibleDC(hdc);
    let bitmap = CreateCompatibleBitmap(hdc, width, height);
    let old = SelectObject(mem, bitmap as *mut _);
    let ground = CreateSolidBrush(rgb(0x1A, 0x17, 0x14));
    FillRect(mem, &rect, ground);
    DeleteObject(ground as *mut _);

    let snapshot = lock().clone_text();
    let mut y = 48;
    text(mem, 36, y, 28, rgb(0xE4, 0xDC, 0xCF), true, "Audrix");
    y += 44;
    let rule = RECT { left: 36, top: y, right: 108, bottom: y + 2 };
    let oxblood = CreateSolidBrush(rgb(0x7C, 0x30, 0x30));
    FillRect(mem, &rule, oxblood);
    DeleteObject(oxblood as *mut _);
    y += 48;

    if let Some(ms) = snapshot.rtt_ms {
        let number = ms.to_string();
        let width = text(mem, 36, y, 84, rgb(0xE4, 0xDC, 0xCF), true, &number);
        text(mem, 36 + width + 14, y + 52, 22, rgb(0x8A, 0x80, 0x74), true, "ms");
        y += 110;
        if let Some(peer) = snapshot.peer {
            text(mem, 36, y, 18, rgb(0x8A, 0x80, 0x74), false, &peer);
        }
    } else {
        text(mem, 36, y, 36, rgb(0xE4, 0xDC, 0xCF), true, &snapshot.ip);
        y += 64;
        text(mem, 36, y, 18, rgb(0x8A, 0x80, 0x74), false, "รอให้มือถือเลือกเครื่องนี้");
    }

    if let Some(error) = snapshot.error {
        text(mem, 36, height - 96, 16, rgb(0x7C, 0x30, 0x30), false, &error);
    } else if !snapshot.device.is_empty() {
        text(mem, 36, height - 64, 16, rgb(0x8A, 0x80, 0x74), false, &snapshot.device);
    }

    BitBlt(hdc, 0, 0, width, height, mem, 0, 0, SRCCOPY);
    SelectObject(mem, old);
    DeleteObject(bitmap as *mut _);
    DeleteDC(mem);
    EndPaint(hwnd, &paint_struct);
}

struct Snapshot {
    ip: String,
    device: String,
    peer: Option<String>,
    rtt_ms: Option<u32>,
    error: Option<String>,
}

impl AppState {
    fn clone_text(&self) -> Snapshot {
        Snapshot {
            ip: self.ip.clone(),
            device: self.device.clone(),
            peer: self.peer.clone(),
            rtt_ms: self.rtt_ms,
            error: self.error.clone(),
        }
    }
}

unsafe fn text(hdc: HDC, x: i32, y: i32, size: i32, color: u32, serif: bool, value: &str) -> i32 {
    let face = if serif { wide("Palatino Linotype") } else { wide("Segoe UI") };
    let font = CreateFontW(
        -size,
        0,
        0,
        0,
        FW_NORMAL as i32,
        0,
        0,
        0,
        DEFAULT_CHARSET as u32,
        OUT_DEFAULT_PRECIS as u32,
        CLIP_DEFAULT_PRECIS as u32,
        CLEARTYPE_QUALITY as u32,
        (DEFAULT_PITCH | FF_DONTCARE) as u32,
        face.as_ptr(),
    );
    let old = SelectObject(hdc, font as *mut _);
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, color);
    let mut rect = RECT { left: x, top: y, right: x + 520, bottom: y + size + 16 };
    let wide_value = wide(value);
    let chars = wide_value.len() as i32 - 1;
    DrawTextW(hdc, wide_value.as_ptr(), chars, &mut rect, DT_LEFT | DT_SINGLELINE);
    let mut extent: SIZE = std::mem::zeroed();
    GetTextExtentPoint32W(hdc, wide_value.as_ptr(), chars, &mut extent);
    SelectObject(hdc, old);
    DeleteObject(font as *mut _);
    let _ = CLR_INVALID;
    extent.cx
}

pub fn alert(message: &str) {
    unsafe {
        let text = wide(message);
        let title = wide("Audrix Relay");
        winapi::um::winuser::MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), 0x10);
    }
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | ((g as u32) << 8) | ((b as u32) << 16)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
