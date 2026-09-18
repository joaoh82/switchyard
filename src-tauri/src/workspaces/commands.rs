//! Workspace and harness commands.

use std::path::PathBuf;

use pty_host::{SessionInfo, TermSize};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

use super::Workspaces;
use crate::error::{IpcError, IpcResult};
use crate::git::Git;
use crate::harness::{self, HarnessDef};
use crate::projects::{Projects, Workspace};
use crate::state::{blocking, AppState};
use crate::terminal::{spawn_in_workspace, HarnessRequest, Launch};

/// A harness definition plus whether its command can be found on this machine.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HarnessInfo {
    #[serde(flatten)]
    pub def: HarnessDef,
    /// Where `command` resolved to on the user's `PATH`; `None` if it is not installed.
    pub resolved_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BranchList {
    pub branches: Vec<String>,
    /// The branch to offer first: the remote's default, or the one checked out.
    pub default: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NewWorkspace {
    pub project_id: String,
    /// `None` starts from the project's default branch.
    pub base_branch: Option<String>,
    pub harness: HarnessRequest,
    pub size: TermSize,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreatedWorkspace {
    pub workspace: Workspace,
    pub session: SessionInfo,
}

fn worktree_root(state: &AppState) -> IpcResult<PathBuf> {
    if let Some(dir) = std::env::var_os("SWITCHYARD_WORKTREE_ROOT").filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    // Visible and short on purpose: people look into these folders, and Windows paths are
    // limited. Becomes a setting in M4.
    state
        .env()
        .home_dir()
        .map(|home| home.join("switchyard"))
        .ok_or_else(|| IpcError::new("no_home", "Cannot determine your home directory."))
}

#[tauri::command]
#[specta::specta]
pub async fn harnesses_list(app: AppHandle) -> IpcResult<Vec<HarnessInfo>> {
    blocking(app, |state| {
        let env = state.env();
        let cwd = std::env::current_dir().unwrap_or_default();
        Ok(harness::builtin()
            .into_iter()
            .map(|def| HarnessInfo {
                resolved_path: env
                    .find_program(&def.command, &cwd)
                    .map(|path| path.to_string_lossy().into_owned()),
                def,
            })
            .collect())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn project_branches(app: AppHandle, project_id: String) -> IpcResult<BranchList> {
    blocking(app, move |state| {
        let project = state
            .store
            .project(&project_id)?
            .ok_or_else(|| IpcError::new("unknown_project", "That project no longer exists."))?;
        let git = Git::new(&state.env())?;
        let root = PathBuf::from(project.root_path);
        Ok(BranchList {
            branches: git.branches(&root)?,
            default: git.default_branch(&root)?,
        })
    })
    .await
}

/// The core loop: make a worktree on a new branch and start a harness in it with the user's
/// first message. If the harness cannot start, the worktree and branch are taken back, so a
/// failed attempt leaves no trace.
#[tauri::command]
#[specta::specta]
pub async fn workspace_create(
    app: AppHandle,
    request: NewWorkspace,
) -> IpcResult<CreatedWorkspace> {
    blocking(app, move |state| {
        // Fail before touching git if the harness cannot possibly start.
        harness::find(&request.harness.id)
            .ok_or_else(|| IpcError::new("unknown_harness", "That harness is not configured."))?;

        let git = Git::new(&state.env())?;
        let root = worktree_root(state)?;
        let workspaces = Workspaces {
            store: &state.store,
            git: &git,
            worktree_root: &root,
        };
        let prompt = request.harness.prompt.clone().unwrap_or_default();
        let row =
            workspaces.create(&request.project_id, request.base_branch.as_deref(), &prompt)?;

        match spawn_in_workspace(
            state,
            &row.id,
            Launch::Harness(request.harness),
            request.size,
        ) {
            Ok(session) => Ok(CreatedWorkspace {
                workspace: Projects {
                    store: &state.store,
                    git: &git,
                }
                .describe_workspace(row),
                session,
            }),
            Err(error) => {
                let _ = workspaces.discard(&row);
                Err(error)
            }
        }
    })
    .await
}

/// Remove a workspace's worktree. The branch is kept. Fails with `worktree_dirty` unless `force`.
#[tauri::command]
#[specta::specta]
pub async fn workspace_delete(app: AppHandle, id: String, force: bool) -> IpcResult<()> {
    blocking(app, move |state| {
        let git = Git::new(&state.env())?;
        let root = worktree_root(state)?;
        Workspaces {
            store: &state.store,
            git: &git,
            worktree_root: &root,
        }
        .delete(&id, force)
    })
    .await
}
