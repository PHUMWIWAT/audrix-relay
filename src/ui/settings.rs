use std::fs;
use std::path::PathBuf;

use super::lock;

pub fn load() {
    let Some(path) = file() else {
        return;
    };
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    let mut lines = text.lines();
    let thai = lines.next() != Some("en");
    let gain = lines
        .next()
        .and_then(|line| line.parse::<u32>().ok())
        .unwrap_or(100)
        .min(200);
    let mut state = lock();
    state.thai = thai;
    state.gain_percent = gain;
    drop(state);
    crate::capture::set_gain_percent(gain);
}

pub fn save() {
    let Some(path) = file() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let state = lock();
    let lang = if state.thai { "th" } else { "en" };
    let text = format!("{lang}\n{}\n", state.gain_percent);
    drop(state);
    let _ = fs::write(path, text);
}

fn file() -> Option<PathBuf> {
    let root = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(root).join("AudrixRelay").join("settings.txt"))
}
