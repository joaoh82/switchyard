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
    pub dev: DevFlags,
}

/// Switches for measuring and debugging, read from the environment. Always empty in release
/// builds.
#[derive(Debug, Clone, Default, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevFlags {
    /// `SWITCHYARD_BENCH`: a shell script to run in a terminal while frame times are recorded;
    /// the app prints the result and exits. See `docs/design/07-terminal-benchmarks.md`.
    pub bench: Option<String>,
    /// `SWITCHYARD_RENDERER`: force the terminal renderer (`webgl` or `dom`).
    pub renderer: Option<String>,
}

impl DevFlags {
    fn from_env() -> Self {
        if !cfg!(debug_assertions) {
            return Self::default();
        }
        let var = |name| std::env::var(name).ok().filter(|v: &String| !v.is_empty());
        Self {
            bench: var("SWITCHYARD_BENCH"),
            renderer: var("SWITCHYARD_RENDERER"),
        }
    }
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
        dev: DevFlags::from_env(),
    }
}

/// Receives the result of a `SWITCHYARD_BENCH` run, prints it as one line of JSON and quits.
#[tauri::command]
#[specta::specta]
pub fn bench_report(app: tauri::AppHandle, report: String) {
    if cfg!(debug_assertions) {
        println!("SWITCHYARD_BENCH_RESULT {report}");
        app.exit(0);
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
