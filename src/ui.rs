#![allow(unsafe_op_in_unsafe_fn)]

use std::sync::{Mutex, OnceLock};

use winapi::shared::minwindef::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use winapi::shared::windef::{HBRUSH, HDC, HICON, HWND, POINT, RECT, SIZE};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::shellapi::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use winapi::um::wingdi::{
    BitBlt, CreateBitmap, CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection, CreateFontW,
    CreateSolidBrush, DeleteDC,
    DeleteObject, GetTextExtentPoint32W, SelectObject, SetBkMode, SetTextColor, StretchDIBits,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CLR_INVALID, DIB_RGB_COLORS, DEFAULT_CHARSET,
    DEFAULT_PITCH, FF_DONTCARE, FW_NORMAL, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
    CLEARTYPE_QUALITY, SRCCOPY, TRANSPARENT,
};
use winapi::um::winuser::{
    AppendMenuW, BeginPaint, CreateIconIndirect, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DestroyMenu, DestroyWindow, DispatchMessageW, DrawTextW, EndPaint, FillRect, GetClientRect,
    GetCursorPos, GetMessageW, InvalidateRect, IsWindowVisible, LoadCursorW, LoadIconW, PostQuitMessage,
    RegisterClassW, SendMessageW, SetForegroundWindow, SetProcessDPIAware, SetTimer, ShowWindow,
    TrackPopupMenu, TranslateMessage, CW_USEDEFAULT, DT_LEFT, DT_SINGLELINE, ICONINFO, ICON_BIG,
    ICON_SMALL, IDC_ARROW, IDI_APPLICATION,
    MF_SEPARATOR, MF_STRING, SW_HIDE, SW_SHOW, TPM_RIGHTALIGN, WM_CLOSE, WM_COMMAND, WM_DESTROY,
    WM_SETICON,
    WM_LBUTTONUP, WM_PAINT, WM_RBUTTONUP, WM_TIMER, WM_USER, WNDCLASSW, WS_CAPTION, WS_MINIMIZEBOX,
    WS_OVERLAPPED, WS_SYSMENU,
};

const WM_TRAY: u32 = WM_USER + 1;
const MENU_SHOW: usize = 1;
const MENU_HIDE: usize = 2;
const MENU_EXIT: usize = 3;

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
    SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, icon as isize);
    SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, icon as isize);
    add_tray(hwnd, icon);
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
            WM_CLOSE => {
                ShowWindow(hwnd, SW_HIDE);
                0
            }
            WM_TRAY => {
                match lparam as u32 {
                    WM_LBUTTONUP => toggle_window(hwnd),
                    WM_RBUTTONUP => tray_menu(hwnd),
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
                        remove_tray(hwnd);
                        DestroyWindow(hwnd);
                    }
                    _ => {}
                }
                0
            }
            WM_DESTROY => {
                remove_tray(hwnd);
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
    draw_logo(mem, 36, 36, 84);
    text(mem, 136, 52, 28, rgb(0xE4, 0xDC, 0xCF), true, "Audrix");
    let rule = RECT { left: 136, top: 96, right: 208, bottom: 98 };
    let oxblood = CreateSolidBrush(rgb(0x7C, 0x30, 0x30));
    FillRect(mem, &rule, oxblood);
    DeleteObject(oxblood as *mut _);
    let mut y = 156;

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

const LOGO: &[u8] = include_bytes!("../assets/logo-mark.bmp");

unsafe fn draw_logo(hdc: HDC, x: i32, y: i32, size: i32) {
    if LOGO.len() < 54 {
        return;
    }
    let mut size_bytes = [0u8; 4];
    size_bytes.copy_from_slice(&LOGO[18..22]);
    let width = i32::from_le_bytes(size_bytes);
    size_bytes.copy_from_slice(&LOGO[22..26]);
    let height = i32::from_le_bytes(size_bytes);
    size_bytes.copy_from_slice(&LOGO[10..14]);
    let offset = u32::from_le_bytes(size_bytes) as usize;
    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = width;
    info.bmiHeader.biHeight = height;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;
    StretchDIBits(
        hdc,
        x,
        y,
        size,
        size,
        0,
        0,
        width,
        height,
        LOGO[offset..].as_ptr() as *const _,
        &info,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | ((g as u32) << 8) | ((b as u32) << 16)
}

unsafe fn app_icon() -> HICON {
    let icon = make_icon();
    if icon.is_null() {
        LoadIconW(std::ptr::null_mut(), IDI_APPLICATION)
    } else {
        icon
    }
}

unsafe fn add_tray(hwnd: HWND, icon: HICON) {
    let mut data: NOTIFYICONDATAW = std::mem::zeroed();
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = 1;
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = WM_TRAY;
    data.hIcon = icon;
    let tip = wide("Audrix Relay");
    let n = tip.len().min(data.szTip.len());
    data.szTip[..n].copy_from_slice(&tip[..n]);
    Shell_NotifyIconW(NIM_ADD, &mut data);
}

unsafe fn remove_tray(hwnd: HWND) {
    let mut data: NOTIFYICONDATAW = std::mem::zeroed();
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = 1;
    Shell_NotifyIconW(NIM_DELETE, &mut data);
}

unsafe fn toggle_window(hwnd: HWND) {
    if IsWindowVisible(hwnd) == 0 {
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
    } else {
        ShowWindow(hwnd, SW_HIDE);
    }
}

unsafe fn tray_menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    let show = wide("แสดง");
    let hide = wide("ซ่อน");
    let exit = wide("ออก");
    AppendMenuW(menu, MF_STRING, MENU_SHOW, show.as_ptr());
    AppendMenuW(menu, MF_STRING, MENU_HIDE, hide.as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
    AppendMenuW(menu, MF_STRING, MENU_EXIT, exit.as_ptr());
    let mut point: POINT = std::mem::zeroed();
    GetCursorPos(&mut point);
    SetForegroundWindow(hwnd);
    TrackPopupMenu(menu, TPM_RIGHTALIGN, point.x, point.y, 0, hwnd, std::ptr::null());
    DestroyMenu(menu);
}

unsafe fn make_icon() -> HICON {
    if LOGO.len() < 54 {
        return std::ptr::null_mut();
    }
    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = 32;
    info.bmiHeader.biHeight = -32;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;
    let mut bits: *mut winapi::ctypes::c_void = std::ptr::null_mut();
    let color = CreateDIBSection(
        std::ptr::null_mut(),
        &info,
        DIB_RGB_COLORS,
        &mut bits,
        std::ptr::null_mut(),
        0,
    );
    if color.is_null() || bits.is_null() {
        return std::ptr::null_mut();
    }
    let pixels = std::slice::from_raw_parts_mut(bits as *mut u8, 32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            let src_x = x * 4;
            let src_y = y * 4;
            let src_row = 127 - src_y;
            let src = 54 + ((src_row * 128 + src_x) * 4) as usize;
            let dst = (y * 32 + x) * 4;
            if src + 4 <= LOGO.len() {
                pixels[dst] = LOGO[src];
                pixels[dst + 1] = LOGO[src + 1];
                pixels[dst + 2] = LOGO[src + 2];
                pixels[dst + 3] = 255;
            }
        }
    }
    let mask_bits = [0u8; 128];
    let mask = CreateBitmap(32, 32, 1, 1, mask_bits.as_ptr() as *const _);
    let mut icon_info: ICONINFO = std::mem::zeroed();
    icon_info.fIcon = 1;
    icon_info.hbmColor = color;
    icon_info.hbmMask = mask;
    let icon = CreateIconIndirect(&mut icon_info);
    DeleteObject(color as *mut _);
    DeleteObject(mask as *mut _);
    icon
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
