use winapi::shared::windef::{HDC, RECT};
use winapi::um::wingdi::{CreateSolidBrush, DeleteObject};
use winapi::um::winuser::FillRect;

use super::rgb;

#[derive(Clone, Copy)]
pub struct Bar {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub fn contains(bar: Bar, x: i32, y: i32) -> bool {
    x >= bar.x && x <= bar.x + bar.w && y >= bar.y - 10 && y <= bar.y + bar.h + 10
}

pub fn percent_at(bar: Bar, x: i32) -> u32 {
    let t = ((x - bar.x) as f32 / bar.w as f32).clamp(0.0, 1.0);
    (t * 200.0).round() as u32
}

pub unsafe fn draw(hdc: HDC, bar: Bar, percent: u32) {
    let track = RECT {
        left: bar.x,
        top: bar.y + bar.h / 2,
        right: bar.x + bar.w,
        bottom: bar.y + bar.h / 2 + 2,
    };
    let quiet = CreateSolidBrush(rgb(0x3A, 0x34, 0x2E));
    FillRect(hdc, &track, quiet);
    DeleteObject(quiet as *mut _);
    let fill_w = (bar.w as f32 * percent as f32 / 200.0) as i32;
    let fill = RECT {
        left: bar.x,
        top: bar.y + bar.h / 2,
        right: bar.x + fill_w,
        bottom: bar.y + bar.h / 2 + 2,
    };
    let ivory = CreateSolidBrush(rgb(0xE4, 0xDC, 0xCF));
    FillRect(hdc, &fill, ivory);
    let thumb = RECT {
        left: bar.x + fill_w - 6,
        top: bar.y,
        right: bar.x + fill_w + 6,
        bottom: bar.y + bar.h,
    };
    FillRect(hdc, &thumb, ivory);
    DeleteObject(ivory as *mut _);
}
