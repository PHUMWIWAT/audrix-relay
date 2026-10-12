use winapi::shared::windef::{HDC, HWND, RECT, SIZE};
use winapi::um::wingdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush, DeleteDC,
    DeleteObject, GetTextExtentPoint32W, SelectObject, SetBkMode, SetTextColor, CLR_INVALID,
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, FF_DONTCARE, FW_NORMAL,
    OUT_DEFAULT_PRECIS, SRCCOPY, TRANSPARENT,
};
use winapi::um::winuser::{
    BeginPaint, DrawTextW, EndPaint, FillRect, GetClientRect, DT_LEFT, DT_SINGLELINE,
};

use super::{lock, rgb, wide, Snapshot};

pub unsafe fn paint(hwnd: HWND) {
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

    let state = lock();
    let snapshot = Snapshot {
        ip: state.ip.clone(),
        device: state.device.clone(),
        peer: state.peer.clone(),
        rtt_ms: state.rtt_ms,
        error: state.error.clone(),
        gain_percent: state.gain_percent,
        thai: state.thai,
        screen: state.screen,
        notice: state.notice.clone(),
    };
    drop(state);
    super::screens::draw(mem, width, height, &snapshot);

    BitBlt(hdc, 0, 0, width, height, mem, 0, 0, SRCCOPY);
    SelectObject(mem, old);
    DeleteObject(bitmap as *mut _);
    DeleteDC(mem);
    EndPaint(hwnd, &paint_struct);
}

pub unsafe fn label(hdc: HDC, x: i32, y: i32, size: i32, color: u32, value: &str) -> i32 {
    text(hdc, x, y, size, color, false, value)
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
