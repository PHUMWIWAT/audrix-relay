use winapi::shared::windef::{HDC, HICON};
use winapi::um::wingdi::{
    CreateBitmap, CreateDIBSection, DeleteObject, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, SRCCOPY,
};
use winapi::um::winuser::{CreateIconIndirect, ICONINFO};

const LOGO: &[u8] = include_bytes!("../../assets/logo-mark.bmp");

pub unsafe fn draw(hdc: HDC, x: i32, y: i32, size: i32) {
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

pub unsafe fn icon() -> HICON {
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
