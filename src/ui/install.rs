use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn install() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let dir = install_dir()?;
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let target = dir.join("AudrixRelay.exe");
    if exe != target {
        fs::copy(&exe, &target).map_err(|err| err.to_string())?;
    }
    let ico = dir.join("AudrixRelay.ico");
    fs::write(&ico, super::logo::ico_bytes()).map_err(|err| err.to_string())?;
    shortcut(&target, &ico)?;
    Ok(target.display().to_string())
}

pub fn uninstall() -> Result<(), String> {
    if let Some(link) = shortcut_path() {
        let _ = fs::remove_file(link);
    }
    Ok(())
}

fn install_dir() -> Result<PathBuf, String> {
    let root = std::env::var("LOCALAPPDATA").map_err(|err| err.to_string())?;
    Ok(PathBuf::from(root).join("AudrixRelay"))
}

fn shortcut_path() -> Option<PathBuf> {
    let root = std::env::var("APPDATA").ok()?;
    Some(
        PathBuf::from(root)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Audrix Relay.lnk"),
    )
}

fn shortcut(exe: &Path, ico: &Path) -> Result<(), String> {
    let link = shortcut_path().ok_or("no start menu path")?;
    if let Some(dir) = link.parent() {
        fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    }
    let ps = format!(
        "$s = New-Object -ComObject WScript.Shell; $l = $s.CreateShortcut('{}'); $l.TargetPath = '{}'; $l.WorkingDirectory = '{}'; $l.IconLocation = '{},0'; $l.Save()",
        link.display().to_string().replace('\'', "''"),
        exe.display().to_string().replace('\'', "''"),
        exe.parent().unwrap_or(Path::new(".")).display().to_string().replace('\'', "''"),
        ico.display().to_string().replace('\'', "''"),
    );
    let status = Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps])
        .status()
        .map_err(|err| err.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("could not create the Start menu shortcut".into())
    }
}
