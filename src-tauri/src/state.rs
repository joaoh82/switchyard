use std::sync::{Arc, Mutex, PoisonError};

use pty_host::PtyHost;

use crate::env::ShellEnv;

/// Everything the commands share. Managed by Tauri, created in `setup`.
pub struct AppState {
    pub host: PtyHost,
    env: Mutex<Option<Arc<ShellEnv>>>,
}

impl AppState {
    pub fn new(host: PtyHost) -> Self {
        Self {
            host,
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
