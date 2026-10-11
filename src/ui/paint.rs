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

use super::{lock, logo, rgb, wide, Snapshot};

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
    };
    drop(state);

    logo::draw(mem, 36, 36, 84);
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
