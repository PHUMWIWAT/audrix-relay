use winapi::shared::windef::{HDC, RECT};
use winapi::um::wingdi::{CreateSolidBrush, DeleteObject};
use winapi::um::winuser::FillRect;
use super::rgb;

pub fn contains(x: i32, y: i32, width: i32, height: i32) -> bool {
    let (sx, sy, sw, sh) = bounds(width, height);
    x >= sx && x <= sx + sw && y >= sy - 8 && y <= sy + sh + 8
}

pub fn percent_at(x: i32, width: i32) -> u32 {
    let (sx, _, sw, _) = bounds(width, 0);
    let t = ((x - sx) as f32 / sw as f32).clamp(0.0, 1.0);
    (t * 200.0).round() as u32
}

pub fn lang_at(x: i32, y: i32, _width: i32) -> Option<bool> {
    if y < 36 || y > 72 {
        return None;
    }
    if (300..340).contains(&x) {
        Some(false)
    } else if (348..388).contains(&x) {
        Some(true)
    } else {
        None
    }
}

pub unsafe fn draw(hdc: HDC, width: i32, height: i32, percent: u32, thai: bool) {
    let (x, y, w, h) = bounds(width, height);
    let track = RECT { left: x, top: y + h / 2, right: x + w, bottom: y + h / 2 + 2 };
    let quiet = CreateSolidBrush(rgb(0x3A, 0x34, 0x2E));
    FillRect(hdc, &track, quiet);
    DeleteObject(quiet as *mut _);
    let fill_w = (w as f32 * percent as f32 / 200.0) as i32;
    let fill = RECT { left: x, top: y + h / 2, right: x + fill_w, bottom: y + h / 2 + 2 };
    let ivory = CreateSolidBrush(rgb(0xE4, 0xDC, 0xCF));
    FillRect(hdc, &fill, ivory);
    let thumb = RECT {
        left: x + fill_w - 6,
        top: y,
        right: x + fill_w + 6,
        bottom: y + h,
    };
    FillRect(hdc, &thumb, ivory);
    DeleteObject(ivory as *mut _);
    let label = if thai { "เสียง" } else { "Volume" };
    super::paint::label(hdc, x, y - 28, 16, rgb(0x8A, 0x80, 0x74), &format!("{label}  {percent}%"));
}

fn bounds(width: i32, height: i32) -> (i32, i32, i32, i32) {
    (36, height - 78, width - 72, 18)
}
