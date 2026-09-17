//! The IPC surface. Every function here is callable from the webview.

use serde::Serialize;
use specta::Type;

/// Static facts about the running app, shown in the UI and useful in bug reports.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    /// `linux`, `macos` or `windows`.
    pub os: String,
    pub arch: String,
    pub debug: bool,
}

#[tauri::command]
#[specta::specta]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: "Switchyard".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        debug: cfg!(debug_assertions),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_reports_this_build() {
        let info = app_info();
        assert_eq!(info.name, "Switchyard");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert!(["linux", "macos", "windows"].contains(&info.os.as_str()));
        assert!(!info.arch.is_empty());
    }
}
