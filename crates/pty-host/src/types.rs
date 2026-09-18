//! The message types that cross the host boundary. Everything here is plain serialisable data:
//! today it crosses a function call, later a local socket.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Opaque, globally unique session identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(transparent)]
pub struct SessionId(pub String);

impl SessionId {
    pub(crate) fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Identifies one attachment (one viewer) of a session.
pub type AttachmentId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TermSize {
    pub cols: u16,
    pub rows: u16,
}

impl TermSize {
    pub const DEFAULT: Self = Self { cols: 80, rows: 24 };

    /// A PTY with a zero dimension misbehaves on every platform; clamp instead of failing.
    pub(crate) fn sanitized(self) -> Self {
        Self {
            cols: self.cols.max(2),
            rows: self.rows.max(1),
        }
    }
}

impl Default for TermSize {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Everything needed to start a process in a PTY. The program is executed directly with `args`
/// as its argv — never through a shell — so no quoting or escaping is involved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LaunchPlan {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory. `None` inherits the host's.
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    /// Environment variables to set, applied on top of the base environment.
    #[serde(default)]
    pub env: Vec<(String, String)>,
    /// Start from an empty environment instead of the host's, so `env` is the whole environment.
    #[serde(default)]
    pub clear_env: bool,
    #[serde(default)]
    pub size: TermSize,
    /// Opaque metadata the host stores and reports back in [`SessionInfo`], never interpreting
    /// it. Lets a client that reconnects — a reloaded webview today, the app attaching to a
    /// daemon later — work out what each session belongs to.
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ExitInfo {
    pub code: u32,
    pub success: bool,
    /// Name of the terminating signal, where the platform reports one.
    pub signal: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum SessionState {
    Running,
    Exited { exit: ExitInfo },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: SessionId,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub pid: Option<u32>,
    pub size: TermSize,
    pub labels: BTreeMap<String, String>,
    pub state: SessionState,
    /// Milliseconds since the session last produced output (saturating). Drives "busy / waiting" indicators
    /// without anyone having to parse what the program printed.
    pub idle_ms: u32,
}

/// Host-wide notifications, delivered to the sink given to [`crate::PtyHost::new`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum HostEvent {
    /// The session's process ended and all of its output has been delivered.
    Exited { id: SessionId, exit: ExitInfo },
}

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("no such session: {0}")]
    UnknownSession(SessionId),
    #[error("session has exited: {0}")]
    SessionExited(SessionId),
    #[error("could not open a pseudo-terminal: {0}")]
    OpenPty(String),
    #[error("could not start `{program}`: {reason}")]
    Spawn { program: String, reason: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, HostError>;
