//! Switchyard core.
//!
//! The frontend holds no truth: state lives here and the webview renders it. `commands` is the
//! whole IPC surface and stays thin — real work belongs in the domain modules.

mod commands;
mod env;
mod error;
mod state;
mod terminal;

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
        .events(collect_events![terminal::PtyHostEvent])
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
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            let handle = app.handle().clone();
            let host = pty_host::PtyHost::new(std::sync::Arc::new(move |event| {
                let _ = terminal::PtyHostEvent(event).emit(&handle);
            }));
            app.manage(state::AppState::new(host));

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
