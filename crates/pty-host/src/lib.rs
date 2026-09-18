//! The PTY host: owns pseudo-terminal sessions and the processes running in them.
//!
//! This crate deliberately knows nothing about Tauri, windows or the UI. Its API is shaped like
//! messages — serialisable requests in, byte streams and events out, sessions addressed by id —
//! so the same host can run in-process (today) or as a background daemon that the app attaches
//! to (later) without anything above this boundary changing.
//!
//! ```text
//! spawn(LaunchPlan) -> SessionInfo        list() -> [SessionInfo]
//! attach(id, sink)  -> AttachmentId       detach(id, attachment)
//! write(id, bytes) / paste(id, text)       resize(id, size)
//! kill(id)                                remove(id)
//! events: HostEvent::{Exited, Busy, Quiet}
//! ```
//!
//! Each session keeps a headless terminal (`vt100`) fed with everything the process prints. A
//! viewer that attaches — for the first time or the tenth — first receives a snapshot that
//! repaints scrollback and screen, then the live stream, with nothing lost or duplicated between
//! the two.

mod session;
mod snapshot;
mod types;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use session::Session;
pub use types::{
    AttachmentId, ExitInfo, HostError, HostEvent, LaunchPlan, Result, SessionId, SessionInfo,
    SessionState, TermSize,
};

/// Receives a session's output: first one snapshot, then live batches. Return `false` to detach.
///
/// Called on the session's pump thread while it holds the session lock, so it must be quick and
/// must not call back into the host.
pub type OutputSink = Box<dyn FnMut(&[u8]) -> bool + Send>;

/// Receives host-wide events. Called from session threads.
pub type EventSink = Arc<dyn Fn(HostEvent) + Send + Sync>;

/// How long a session must print nothing before it counts as quiet. Long enough that a program
/// pausing between lines stays "busy"; agents animate a spinner while they think, so for them
/// silence really does mean "waiting for you".
pub const DEFAULT_QUIET_AFTER: Duration = Duration::from_secs(3);

pub struct PtyHost {
    sessions: Mutex<HashMap<SessionId, Arc<Session>>>,
    events: EventSink,
    quiet_after: Duration,
}

impl PtyHost {
    pub fn new(events: EventSink) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            events,
            quiet_after: DEFAULT_QUIET_AFTER,
        }
    }

    /// Use a different quiet period for sessions spawned from now on.
    pub fn with_quiet_after(mut self, quiet_after: Duration) -> Self {
        self.quiet_after = quiet_after;
        self
    }

    /// Start `plan.program` in a new PTY.
    pub fn spawn(&self, plan: LaunchPlan) -> Result<SessionInfo> {
        let session = Session::spawn(plan, Arc::clone(&self.events), self.quiet_after)?;
        let info = session.info();
        self.lock().insert(info.id.clone(), session);
        Ok(info)
    }

    /// Start receiving a session's output. The sink is handed a snapshot of the current terminal
    /// state before this returns, then live output. Works on exited sessions too, which is how
    /// their final screen stays viewable.
    pub fn attach(&self, id: &SessionId, sink: OutputSink) -> Result<AttachmentId> {
        Ok(self.get(id)?.attach(sink))
    }

    pub fn detach(&self, id: &SessionId, attachment: AttachmentId) -> Result<()> {
        self.get(id)?.detach(attachment);
        Ok(())
    }

    /// Send input to the session's process.
    pub fn write(&self, id: &SessionId, data: &[u8]) -> Result<()> {
        self.get(id)?.write(data)
    }

    /// Paste text into the session, bracketed if the program enabled bracketed paste.
    pub fn paste(&self, id: &SessionId, text: &str) -> Result<()> {
        self.get(id)?.paste(text)
    }

    pub fn resize(&self, id: &SessionId, size: TermSize) -> Result<()> {
        self.get(id)?.resize(size)
    }

    /// Ask the session's process to terminate. `Exited` follows once it has.
    pub fn kill(&self, id: &SessionId) -> Result<()> {
        self.get(id)?.kill()
    }

    /// Forget a session, killing it first if it is still running.
    pub fn remove(&self, id: &SessionId) -> Result<()> {
        let session = self
            .lock()
            .remove(id)
            .ok_or_else(|| HostError::UnknownSession(id.clone()))?;
        // Already-exited sessions report an error here; either way it is gone.
        let _ = session.kill();
        Ok(())
    }

    pub fn info(&self, id: &SessionId) -> Result<SessionInfo> {
        Ok(self.get(id)?.info())
    }

    pub fn list(&self) -> Vec<SessionInfo> {
        let mut all: Vec<_> = self.lock().values().map(|s| s.info()).collect();
        all.sort_by(|a, b| a.id.cmp(&b.id));
        all
    }

    fn get(&self, id: &SessionId) -> Result<Arc<Session>> {
        self.lock()
            .get(id)
            .cloned()
            .ok_or_else(|| HostError::UnknownSession(id.clone()))
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<SessionId, Arc<Session>>> {
        self.sessions.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Drop for PtyHost {
    fn drop(&mut self) {
        for session in self.lock().values() {
            let _ = session.kill();
        }
    }
}
