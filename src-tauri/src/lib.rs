//! Switchyard core.
//!
//! The frontend holds no truth: state lives here and the webview renders it. `commands` is the
//! whole IPC surface and stays thin — real work belongs in the domain modules.

mod changes;
mod commands;
mod env;
mod error;
mod git;
mod harness;
mod projects;
mod sessions;
mod settings;
mod state;
mod store;
mod terminal;
mod workspaces;

pub use env::print_env_and_exit_if_asked;

use tauri::Manager;
use tauri_specta::{collect_commands, collect_events, Builder, Event};

/// Where the generated TypeScript bindings live, relative to this crate.
#[cfg(any(debug_assertions, test))]
const BINDINGS_PATH: &str = "../src/lib/bindings.ts";

fn ipc_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::app_info,
            commands::bench_report,
            projects::commands::projects_list,
            projects::commands::project_open,
            projects::commands::project_create,
            projects::commands::project_remove,
            projects::commands::projects_reorder,
            projects::commands::ui_state_load,
            projects::commands::ui_state_save,
            workspaces::commands::harnesses_list,
            workspaces::commands::harness_save,
            workspaces::commands::harness_reset,
            workspaces::commands::harness_preview,
            workspaces::commands::harness_test,
            workspaces::commands::settings_get,
            workspaces::commands::settings_save_workspaces,
            workspaces::commands::settings_save_general,
            workspaces::commands::project_branches,
            workspaces::commands::workspace_create,
            workspaces::commands::workspace_delete,
            changes::commands::workspace_changes,
            changes::commands::workspace_diff,
            changes::commands::workspace_files,
            changes::commands::workspace_file,
            changes::commands::workspace_watch,
            changes::commands::open_in_editor,
            sessions::sessions_list,
            sessions::session_resume,
            sessions::session_fork,
            sessions::session_forget,
            workspaces::commands::workspace_archive,
            workspaces::commands::workspace_restore,
            workspaces::commands::workspace_rename,
            terminal::env_info,
            terminal::pty_spawn,
            terminal::pty_attach,
            terminal::pty_detach,
            terminal::pty_write,
            terminal::pty_resize,
            terminal::pty_kill,
            terminal::pty_close,
            terminal::pty_list,
        ])
        .events(collect_events![
            terminal::PtyHostEvent,
            changes::commands::WorkspaceFilesChanged
        ])
}

/// Release builds never write bindings: there is no source tree next to an installed app.
#[cfg(any(debug_assertions, test))]
fn export_bindings(builder: &Builder<tauri::Wry>) {
    builder
        .export(
            specta_typescript::Typescript::default().header("// @ts-nocheck\n/* eslint-disable */"),
            BINDINGS_PATH,
        )
        .expect("failed to export TypeScript bindings");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = ipc_builder();

    // Keep the checked-in bindings fresh while developing. CI verifies they are not stale.
    #[cfg(debug_assertions)]
    export_bindings(&builder);

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            let handle = app.handle().clone();
            let host = pty_host::PtyHost::new(std::sync::Arc::new(move |event| {
                // Settle the record first, so a client reacting to the event reads the truth.
                if let pty_host::HostEvent::Exited { id, exit } = &event {
                    if let Some(state) = handle.try_state::<state::AppState>() {
                        let _ = state
                            .store
                            .end_session_by_pty(&id.0, Some(i64::from(exit.code)));
                    }
                }
                let _ = terminal::PtyHostEvent(event).emit(&handle);
            }));
            // `SWITCHYARD_DATA_DIR` keeps experiments and tests away from the real database.
            let data_dir = match std::env::var_os("SWITCHYARD_DATA_DIR") {
                Some(dir) if !dir.is_empty() => std::path::PathBuf::from(dir),
                _ => app.path().app_data_dir()?,
            };
            let database = data_dir.join("switchyard.db");
            let store = store::Store::open(&database)
                .map_err(|e| format!("cannot open {}: {e}", database.display()))?;
            // Nothing is running yet: sessions that claim to be died with the previous run.
            store
                .end_interrupted_sessions()
                .map_err(|e| format!("cannot tidy session records: {e}"))?;
            // Settings sit next to the database when the data dir is overridden, otherwise in
            // the OS config directory.
            let config_dir = match std::env::var_os("SWITCHYARD_DATA_DIR") {
                Some(dir) if !dir.is_empty() => std::path::PathBuf::from(dir),
                _ => app.path().app_config_dir()?,
            };
            let settings = settings::SettingsFile::load(config_dir.join("settings.toml"));
            app.manage(state::AppState::new(host, store, settings));

            // Warm the login-shell environment now, so the first terminal doesn't wait for it.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                handle.state::<state::AppState>().env();
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Switchyard");
}

#[cfg(test)]
mod tests {
    /// `bun run bindings` runs this to regenerate `src/lib/bindings.ts` without launching the app.
    #[test]
    fn export_bindings() {
        super::export_bindings(&super::ipc_builder());
    }
}
