//! User-agent dialogs, as webcore's `platform::dialogs` exposes them.

use std::path::PathBuf;
use std::process::Command;

use serde_json::json;

use crate::page;

pub fn alert(message: &str) {
    page::read::<bool>(page::visible_page(), "window_alert", json!([message]));
}

pub fn confirm(message: &str) -> bool {
    page::read(page::visible_page(), "window_confirm", json!([message]))
}

pub fn open_file(
    _title: &str,
    _filters: &[(String, Vec<String>)],
    _directory: &str,
    _multiple: bool,
) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let prompt = if _multiple {
            "choose file with multiple selections allowed"
        } else {
            "choose file"
        };
        let script = format!("POSIX path of ({prompt})");
        if let Ok(output) = Command::new("osascript").arg("-e").arg(&script).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return stdout
                    .trim()
                    .split(", ")
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
                    .collect();
            }
        }
    }
    Vec::new()
}

pub fn save_file(
    _title: &str,
    _filters: &[(String, Vec<String>)],
    _directory: &str,
    _suggested: &str,
) -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let default_name = if !_suggested.is_empty() {
            format!(" default name \"{}\"", _suggested.replace('"', "\\\""))
        } else {
            String::new()
        };
        let script = format!("POSIX path of (choose file name{default_name})");
        if let Ok(output) = Command::new("osascript").arg("-e").arg(&script).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }
    }
    None
}

pub fn pick_directory(_title: &str, _directory: &str) -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let script = "POSIX path of (choose folder)";
        if let Ok(output) = Command::new("osascript").arg("-e").arg(&script).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }
    }
    None
}
