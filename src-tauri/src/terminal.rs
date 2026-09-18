//! Terminal commands: a thin adapter between the webview and the PTY host.

use std::path::PathBuf;
use std::sync::Arc;

use pty_host::{AttachmentId, HostError, HostEvent, LaunchPlan, SessionId, SessionInfo, TermSize};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::ipc::{Channel, InvokeResponseBody, IpcResponse};
use tauri::{AppHandle, State};

use crate::env::{EnvSource, ShellEnv};
use crate::error::{IpcError, IpcResult};
use crate::state::{blocking, AppState};

/// Emitted for every [`HostEvent`].
#[derive(Debug, Clone, Serialize, Type, tauri_specta::Event)]
pub struct PtyHostEvent(pub HostEvent);

/// Terminal output, sent over the channel as raw bytes rather than JSON: the webview receives
/// an `ArrayBuffer` it can hand straight to xterm.js.
///
/// The generated binding types this as `number[]` because specta only sees the `Vec<u8>`;
/// `src/lib/ipc.ts` corrects that in one place.
#[derive(Debug, Clone, Type)]
#[specta(transparent)]
pub struct RawBytes(Vec<u8>);

impl IpcResponse for RawBytes {
    fn body(self) -> tauri::Result<InvokeResponseBody> {
        Ok(InvokeResponseBody::Raw(self.0))
    }
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SpawnRequest {
    /// Program to run. `None` starts the user's shell.
    pub program: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory. `None` means the home directory. Ignored when `workspace_id` is set.
    pub cwd: Option<String>,
    /// Run inside this workspace: the core looks up its folder (the webview never supplies
    /// paths for this) and labels the session so it can be matched back to the workspace.
    pub workspace_id: Option<String>,
    pub size: TermSize,
}

/// Label carrying the id of the workspace a session belongs to.
pub const WORKSPACE_LABEL: &str = "workspace";

/// What the launch environment looks like, for the status bar and for bug reports.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EnvInfo {
    pub source: EnvSource,
    pub shell: String,
    pub path_entries: u32,
    pub warning: Option<String>,
}

impl From<&ShellEnv> for EnvInfo {
    fn from(env: &ShellEnv) -> Self {
        let entries = env
            .get("PATH")
            .map(|path| std::env::split_paths(path).count())
            .unwrap_or(0);
        Self {
            source: env.source,
            shell: env.default_shell().0,
            path_entries: u32::try_from(entries).unwrap_or(u32::MAX),
            warning: env.warning.clone(),
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn pty_spawn(app: AppHandle, request: SpawnRequest) -> IpcResult<SessionInfo> {
    // Resolving the environment and forking are both blocking.
    blocking(app, move |state| {
        let mut request = request;
        let mut labels = std::collections::BTreeMap::new();
        if let Some(id) = request.workspace_id.clone() {
            let workspace = state.store.workspace(&id)?.ok_or_else(|| {
                IpcError::new("unknown_workspace", "That workspace no longer exists.")
            })?;
            if !std::path::Path::new(&workspace.path).is_dir() {
                return Err(IpcError::new(
                    "workspace_missing",
                    format!("{} does not exist any more.", workspace.path),
                ));
            }
            request.cwd = Some(workspace.path);
            labels.insert(WORKSPACE_LABEL.to_owned(), id);
        }
        let mut plan = launch_plan(&state.env(), request)?;
        plan.labels = labels;
        Ok(state.host.spawn(plan)?)
    })
    .await
}

/// Stream a session into `output`: first a snapshot that repaints the terminal, then live bytes.
#[tauri::command]
#[specta::specta]
pub async fn pty_attach(
    state: State<'_, AppState>,
    id: SessionId,
    output: Channel<RawBytes>,
) -> IpcResult<AttachmentId> {
    let sink = Box::new(move |bytes: &[u8]| {
        // A failed send means the webview side is gone; returning false detaches us.
        output.send(RawBytes(bytes.to_vec())).is_ok()
    });
    Ok(state.host.attach(&id, sink)?)
}

#[tauri::command]
#[specta::specta]
pub async fn pty_detach(
    state: State<'_, AppState>,
    id: SessionId,
    attachment: AttachmentId,
) -> IpcResult<()> {
    Ok(state.host.detach(&id, attachment)?)
}

#[tauri::command]
#[specta::specta]
pub async fn pty_write(state: State<'_, AppState>, id: SessionId, data: String) -> IpcResult<()> {
    Ok(state.host.write(&id, data.as_bytes())?)
}

#[tauri::command]
#[specta::specta]
pub async fn pty_resize(
    state: State<'_, AppState>,
    id: SessionId,
    size: TermSize,
) -> IpcResult<()> {
    Ok(state.host.resize(&id, size)?)
}

#[tauri::command]
#[specta::specta]
pub async fn pty_kill(state: State<'_, AppState>, id: SessionId) -> IpcResult<()> {
    match state.host.kill(&id) {
        // Killing something that already ended is not a failure worth reporting.
        Ok(()) | Err(HostError::SessionExited(_)) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Kill (if needed) and forget a session.
#[tauri::command]
#[specta::specta]
pub async fn pty_close(state: State<'_, AppState>, id: SessionId) -> IpcResult<()> {
    Ok(state.host.remove(&id)?)
}

#[tauri::command]
#[specta::specta]
pub async fn pty_list(state: State<'_, AppState>) -> IpcResult<Vec<SessionInfo>> {
    Ok(state.host.list())
}

#[tauri::command]
#[specta::specta]
pub async fn env_info(app: AppHandle, reload: bool) -> IpcResult<EnvInfo> {
    blocking(app, move |state| {
        let env = if reload {
            state.reload_env()
        } else {
            state.env()
        };
        Ok(EnvInfo::from(&*env))
    })
    .await
}

/// Turn a request into a concrete plan: pick the program, find it on the *user's* `PATH`, and
/// hand the process the user's environment.
fn launch_plan(env: &Arc<ShellEnv>, request: SpawnRequest) -> IpcResult<LaunchPlan> {
    let cwd = request
        .cwd
        .map(PathBuf::from)
        .or_else(|| env.home_dir())
        .filter(|dir| dir.is_dir())
        .or_else(|| std::env::current_dir().ok())
        .ok_or_else(|| IpcError::new("bad_cwd", "no usable working directory"))?;

    let (program, args) = match request.program {
        Some(program) => (program, request.args),
        None => env.default_shell(),
    };

    let resolved = env.find_program(&program, &cwd).ok_or_else(|| {
        IpcError::new(
            "program_not_found",
            format!(
                "`{program}` was not found on PATH ({} entries, from {:?}).",
                EnvInfo::from(&**env).path_entries,
                env.source
            ),
        )
    })?;
    let (program, args) = wrap_for_platform(resolved, args);

    Ok(LaunchPlan {
        program,
        args,
        cwd: Some(cwd),
        env: env
            .vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        // The resolved environment is complete; nothing from the GUI process should leak in.
        clear_env: env.source == EnvSource::LoginShell,
        size: request.size,
        labels: Default::default(),
    })
}

/// npm-installed CLIs on Windows are `.cmd` shims, which only `cmd.exe` can run.
fn wrap_for_platform(program: PathBuf, args: Vec<String>) -> (String, Vec<String>) {
    let path = program.to_string_lossy().into_owned();
    let is_batch = cfg!(windows)
        && program
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"));
    if is_batch {
        let mut wrapped = vec!["/D".to_owned(), "/C".to_owned(), path];
        wrapped.extend(args);
        ("cmd.exe".to_owned(), wrapped)
    } else {
        (path, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process_env() -> Arc<ShellEnv> {
        Arc::new(ShellEnv {
            vars: std::env::vars().collect(),
            source: EnvSource::Process,
            warning: None,
        })
    }

    fn request(program: Option<&str>) -> SpawnRequest {
        SpawnRequest {
            program: program.map(str::to_owned),
            args: vec!["--flag".into()],
            cwd: None,
            workspace_id: None,
            size: TermSize { cols: 80, rows: 24 },
        }
    }

    #[test]
    fn unknown_programs_fail_with_a_useful_error() {
        let err =
            launch_plan(&process_env(), request(Some("switchyard-no-such-program"))).unwrap_err();
        assert_eq!(err.code, "program_not_found");
        assert!(err.message.contains("switchyard-no-such-program"));
    }

    #[test]
    fn no_program_means_the_users_shell_in_a_real_directory() {
        let plan = launch_plan(&process_env(), request(None)).unwrap();
        assert!(PathBuf::from(&plan.program).is_absolute() || plan.program == "cmd.exe");
        assert!(plan.cwd.unwrap().is_dir());
        assert!(
            !plan.clear_env,
            "a process-sourced env is layered on top, not a replacement"
        );
    }

    #[test]
    fn a_missing_cwd_falls_back_instead_of_failing() {
        let mut req = request(None);
        req.cwd = Some("/definitely/not/a/real/dir".into());
        assert!(launch_plan(&process_env(), req)
            .unwrap()
            .cwd
            .unwrap()
            .is_dir());
    }

    #[cfg(not(windows))]
    #[test]
    fn programs_run_directly_off_windows() {
        let (program, args) = wrap_for_platform("/usr/bin/tool.cmd".into(), vec!["a".into()]);
        assert_eq!(
            (program.as_str(), args),
            ("/usr/bin/tool.cmd", vec!["a".to_owned()])
        );
    }
}
