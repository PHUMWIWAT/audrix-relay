use winapi::shared::windef::{HDC, RECT};
use winapi::um::wingdi::{CreateSolidBrush, DeleteObject};
use winapi::um::winuser::FillRect;

use super::{logo, paint, rgb, slider, Snapshot};

pub const NAV_W: i32 = 184;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Settings,
    About,
}

impl Screen {
    pub fn label(self, thai: bool) -> &'static str {
        match (self, thai) {
            (Self::Home, false) => "Status",
            (Self::Home, true) => "สถานะ",
            (Self::Settings, false) => "Settings",
            (Self::Settings, true) => "ตั้งค่า",
            (Self::About, false) => "About",
            (Self::About, true) => "เกี่ยวกับ",
        }
    }
}

pub fn nav_at(x: i32, y: i32) -> Option<Screen> {
    if x >= NAV_W {
        return None;
    }
    let screens = [Screen::Home, Screen::Settings, Screen::About];
    for (index, screen) in screens.into_iter().enumerate() {
        let top = 108 + index as i32 * 44;
        if y >= top && y < top + 40 {
            return Some(screen);
        }
    }
    None
}

pub fn volume_bar() -> slider::Bar {
    slider::Bar { x: 220, y: 168, w: 360, h: 18 }
}

pub fn lang_at(x: i32, y: i32) -> Option<bool> {
    if y < 230 || y > 270 {
        return None;
    }
    if (220..280).contains(&x) {
        Some(false)
    } else if (292..352).contains(&x) {
        Some(true)
    } else {
        None
    }
}

pub fn install_at(x: i32, y: i32) -> bool {
    (220..360).contains(&x) && (250..290).contains(&y)
}

pub fn uninstall_at(x: i32, y: i32) -> bool {
    (376..520).contains(&x) && (250..290).contains(&y)
}

pub unsafe fn draw(hdc: HDC, _width: i32, height: i32, snap: &Snapshot) {
    let rail = RECT { left: 0, top: 0, right: NAV_W, bottom: height };
    let surface = CreateSolidBrush(rgb(0x24, 0x18, 0x16));
    FillRect(hdc, &rail, surface);
    DeleteObject(surface as *mut _);
    logo::draw(hdc, 20, 20, 36);
    paint::label(hdc, 64, 28, 18, rgb(0xE4, 0xDC, 0xCF), "Audrix");
    for (index, screen) in [Screen::Home, Screen::Settings, Screen::About].into_iter().enumerate() {
        let top = 108 + index as i32 * 44;
        if snap.screen == screen {
            let mark = RECT { left: 0, top, right: 3, bottom: top + 40 };
            let accent = CreateSolidBrush(rgb(0x7C, 0x30, 0x30));
            FillRect(hdc, &mark, accent);
            DeleteObject(accent as *mut _);
        }
        let color = if snap.screen == screen { rgb(0xE4, 0xDC, 0xCF) } else { rgb(0x8A, 0x80, 0x74) };
        paint::label(hdc, 24, top + 8, 16, color, screen.label(snap.thai));
    }
    paint::label(hdc, 20, height - 36, 14, rgb(0x8A, 0x80, 0x74), &format!("v{}", env!("CARGO_PKG_VERSION")));

    match snap.screen {
        Screen::Home => home(hdc, snap),
        Screen::Settings => settings(hdc, snap),
        Screen::About => about(hdc, snap),
    }
    if let Some(error) = &snap.error {
        paint::label(hdc, 220, height - 28, 14, rgb(0x7C, 0x30, 0x30), error);
    }
}

unsafe fn home(hdc: HDC, snap: &Snapshot) {
    let (title, sub) = if snap.peer.is_some() {
        if snap.thai {
            ("กำลังส่ง", "เสียงจากเครื่องนี้กำลังไปที่มือถือ")
        } else {
            ("Streaming", "Audio from this PC is going to the phone.")
        }
    } else if snap.thai {
        ("พร้อมเชื่อม", "เปิดแอปบนมือถือ แล้วแตะชื่อเครื่องนี้")
    } else {
        ("Ready", "Open the phone app and tap this PC.")
    };
    paint::label(hdc, 220, 36, 28, rgb(0xE4, 0xDC, 0xCF), title);
    paint::label(hdc, 220, 76, 16, rgb(0x8A, 0x80, 0x74), sub);
    card(hdc, 220, 120, 520, 150);
    let ip_label = if snap.thai { "IP ของเครื่องนี้" } else { "This PC" };
    paint::label(hdc, 240, 136, 14, rgb(0x8A, 0x80, 0x74), ip_label);
    paint::label(hdc, 240, 160, 28, rgb(0xE4, 0xDC, 0xCF), &snap.ip);
    if let Some(ms) = snap.rtt_ms {
        paint::label(hdc, 240, 210, 18, rgb(0xE4, 0xDC, 0xCF), &format!("{ms} ms"));
    }
    if let Some(peer) = &snap.peer {
        paint::label(hdc, 240, 236, 14, rgb(0x8A, 0x80, 0x74), peer);
    }
    if !snap.device.is_empty() {
        paint::label(hdc, 220, 290, 14, rgb(0x8A, 0x80, 0x74), &snap.device);
    }
}

unsafe fn settings(hdc: HDC, snap: &Snapshot) {
    let heading = if snap.thai { "ตั้งค่า" } else { "Settings" };
    paint::label(hdc, 220, 36, 28, rgb(0xE4, 0xDC, 0xCF), heading);
    let volume = if snap.thai { "เสียง" } else { "Volume" };
    paint::label(hdc, 220, 120, 16, rgb(0x8A, 0x80, 0x74), &format!("{volume}  {}%", snap.gain_percent));
    slider::draw(hdc, volume_bar(), snap.gain_percent);
    let lang = if snap.thai { "ภาษา" } else { "Language" };
    paint::label(hdc, 220, 210, 16, rgb(0x8A, 0x80, 0x74), lang);
    let en = if snap.thai { rgb(0x8A, 0x80, 0x74) } else { rgb(0xE4, 0xDC, 0xCF) };
    let th = if snap.thai { rgb(0xE4, 0xDC, 0xCF) } else { rgb(0x8A, 0x80, 0x74) };
    paint::label(hdc, 220, 240, 18, en, "EN");
    paint::label(hdc, 292, 240, 18, th, "TH");
}

unsafe fn about(hdc: HDC, snap: &Snapshot) {
    logo::draw(hdc, 220, 36, 72);
    paint::label(hdc, 308, 48, 28, rgb(0xE4, 0xDC, 0xCF), "Audrix Relay");
    paint::label(hdc, 308, 88, 14, rgb(0x8A, 0x80, 0x74), &format!("v{}", env!("CARGO_PKG_VERSION")));
    let blurb = if snap.thai {
        "ส่งเสียงจาก PC ไปมือถือในวงเดียวกัน"
    } else {
        "Sends this PC's audio to a phone on the same network."
    };
    paint::label(hdc, 220, 140, 16, rgb(0xE4, 0xDC, 0xCF), blurb);
    button(hdc, 220, 250, 140, 40, if snap.thai { "ติดตั้ง" } else { "Install" });
    button(hdc, 376, 250, 144, 40, if snap.thai { "ถอนการติดตั้ง" } else { "Uninstall" });
    if !snap.notice.is_empty() {
        paint::label(hdc, 220, 308, 14, rgb(0x8A, 0x80, 0x74), &snap.notice);
    }
}

unsafe fn card(hdc: HDC, x: i32, y: i32, w: i32, h: i32) {
    let rect = RECT { left: x, top: y, right: x + w, bottom: y + h };
    let brush = CreateSolidBrush(rgb(0x24, 0x18, 0x16));
    FillRect(hdc, &rect, brush);
    DeleteObject(brush as *mut _);
}

unsafe fn button(hdc: HDC, x: i32, y: i32, w: i32, h: i32, label: &str) {
    let rect = RECT { left: x, top: y, right: x + w, bottom: y + h };
    let brush = CreateSolidBrush(rgb(0x7C, 0x30, 0x30));
    FillRect(hdc, &rect, brush);
    DeleteObject(brush as *mut _);
    paint::label(hdc, x + 16, y + 10, 16, rgb(0xE4, 0xDC, 0xCF), label);
}
