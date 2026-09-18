use std::sync::{Arc, Mutex, PoisonError};

use pty_host::PtyHost;
use tauri::{AppHandle, Manager};

use crate::env::ShellEnv;
use crate::error::{IpcError, IpcResult};
use crate::store::Store;

/// Everything the commands share. Managed by Tauri, created in `setup`.
pub struct AppState {
    pub host: PtyHost,
    pub store: Store,
    /// Held while catching up with git, so overlapping project listings reconcile one at a time.
    pub reconciling: Mutex<()>,
    env: Mutex<Option<Arc<ShellEnv>>>,
}

impl AppState {
    pub fn new(host: PtyHost, store: Store) -> Self {
        Self {
            host,
            store,
            reconciling: Mutex::new(()),
            env: Mutex::new(None),
        }
    }

    /// The environment to launch programs in. The first caller resolves it (which can take a
    /// moment: it runs the user's shell startup files) while the others wait on the lock.
    /// Blocking — call from a blocking context.
    pub fn env(&self) -> Arc<ShellEnv> {
        let mut slot = self.env.lock().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(slot.get_or_insert_with(|| Arc::new(ShellEnv::resolve())))
    }

    /// Re-run the login shell, e.g. after the user installed a harness.
    pub fn reload_env(&self) -> Arc<ShellEnv> {
        let mut slot = self.env.lock().unwrap_or_else(PoisonError::into_inner);
        let fresh = Arc::new(ShellEnv::resolve());
        *slot = Some(Arc::clone(&fresh));
        fresh
    }
}

/// Run `f` off the async runtime. Nearly every command blocks — on SQLite, on git, on the login
/// shell — and must not stall the threads that serve the webview.
pub async fn blocking<T: Send + 'static>(
    app: AppHandle,
    f: impl FnOnce(&AppState) -> IpcResult<T> + Send + 'static,
) -> IpcResult<T> {
    tauri::async_runtime::spawn_blocking(move || f(&app.state::<AppState>()))
        .await
        .map_err(|e| IpcError::internal(e.to_string()))?
}
