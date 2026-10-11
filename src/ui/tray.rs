use winapi::shared::windef::{HICON, HWND, POINT};
use winapi::um::shellapi::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use winapi::um::winuser::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, IsWindowVisible, SetForegroundWindow,
    ShowWindow, TrackPopupMenu, MF_SEPARATOR, MF_STRING, SW_HIDE, SW_SHOW, TPM_RIGHTALIGN, WM_USER,
};

use super::{logo, wide};

pub const WM_TRAY: u32 = WM_USER + 1;
pub const MENU_SHOW: usize = 1;
pub const MENU_HIDE: usize = 2;
pub const MENU_EXIT: usize = 3;

pub unsafe fn add(hwnd: HWND, icon: HICON) {
    let shown = if icon.is_null() { logo::icon() } else { icon };
    let mut data: NOTIFYICONDATAW = std::mem::zeroed();
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = 1;
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = WM_TRAY;
    data.hIcon = shown;
    let tip = wide("Audrix Relay");
    let n = tip.len().min(data.szTip.len());
    data.szTip[..n].copy_from_slice(&tip[..n]);
    Shell_NotifyIconW(NIM_ADD, &mut data);
}

pub unsafe fn remove(hwnd: HWND) {
    let mut data: NOTIFYICONDATAW = std::mem::zeroed();
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = 1;
    Shell_NotifyIconW(NIM_DELETE, &mut data);
}

pub unsafe fn toggle(hwnd: HWND) {
    if IsWindowVisible(hwnd) == 0 {
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
    } else {
        ShowWindow(hwnd, SW_HIDE);
    }
}

pub unsafe fn menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    let thai = super::lock().thai;
    let show = wide(if thai { "แสดง" } else { "Show" });
    let hide = wide(if thai { "ซ่อน" } else { "Hide" });
    let exit = wide(if thai { "ออก" } else { "Exit" });
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
