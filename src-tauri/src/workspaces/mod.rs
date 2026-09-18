//! Worktree workspaces: one branch, one folder, one line of work.
//!
//! Everything here is plain git underneath — `git worktree add -b` and `git worktree remove` —
//! so a user can always inspect or undo what Switchyard did with their own tools.

pub mod commands;
mod naming;

use std::path::{Path, PathBuf};

use crate::error::{IpcError, IpcResult};
use crate::git::{Git, GitError};
use crate::store::{Store, WorkspaceRow};

/// Prefix of the branches Switchyard creates: `sy/fix-login-bug`.
pub const BRANCH_PREFIX: &str = "sy";

pub struct Workspaces<'a> {
    pub store: &'a Store,
    pub git: &'a Git,
    /// Worktrees live in `<root>/<project>/<workspace>`, outside the repositories themselves.
    pub worktree_root: &'a Path,
}

impl Workspaces<'_> {
    /// Create a branch from `base` and a worktree for it, named after `prompt`.
    ///
    /// Either everything exists afterwards — branch, folder, database row — or nothing does.
    pub fn create(
        &self,
        project_id: &str,
        base: Option<&str>,
        prompt: &str,
    ) -> IpcResult<WorkspaceRow> {
        let project = self
            .store
            .project(project_id)?
            .ok_or_else(|| IpcError::new("unknown_project", "That project no longer exists."))?;
        let root = PathBuf::from(&project.root_path);
        if !root.is_dir() {
            return Err(IpcError::new(
                "project_missing",
                format!("{} does not exist any more.", project.root_path),
            ));
        }
        if !self.git.has_commits(&root)? {
            return Err(IpcError::new(
                "no_commits",
                "This repository has no commits yet, so there is nothing to branch from. Make a first commit, then try again.",
            ));
        }
        let base = match base {
            Some(base) => base.to_owned(),
            None => self.git.default_branch(&root)?.ok_or_else(|| {
                IpcError::new(
                    "no_base_branch",
                    "Choose a branch to start from: the repository is not on one.",
                )
            })?,
        };

        let project_dir = self
            .worktree_root
            .join(naming::slugify_name(&project.name).unwrap_or_else(|| project.id.clone()));
        let seed = self.store.workspaces()?.len();
        // A name is free only if neither its branch nor its folder exists — including leftovers
        // Switchyard does not know about.
        let mut probe_error = None;
        let name = naming::unique(&naming::base_name(prompt, seed), |candidate| {
            let branch_taken = self
                .git
                .branch_exists(&root, &branch_for(candidate))
                .unwrap_or_else(|e| {
                    probe_error = Some(e);
                    false
                });
            branch_taken || project_dir.join(candidate).exists()
        });
        if let Some(error) = probe_error {
            return Err(error.into());
        }

        let branch = branch_for(&name);
        let path = project_dir.join(&name);
        std::fs::create_dir_all(&project_dir).map_err(|e| {
            IpcError::new(
                "io",
                format!("Cannot create {}: {e}", project_dir.display()),
            )
        })?;
        self.git.worktree_add(&root, &path, &branch, &base)?;

        match self
            .store
            .add_worktree(&project.id, &name, &path.to_string_lossy(), &branch, &base)
        {
            Ok(row) => Ok(row),
            Err(error) => {
                self.undo(&root, &path, &branch);
                Err(error.into())
            }
        }
    }

    /// Undo a workspace that was created a moment ago and never used, e.g. because its harness
    /// failed to start. The branch goes too: it has no commits of its own yet.
    pub fn discard(&self, workspace: &WorkspaceRow) -> IpcResult<()> {
        let root = self.project_root(workspace)?;
        self.store.remove_worktree(&workspace.id)?;
        if let Some(branch) = &workspace.branch {
            self.undo(&root, Path::new(&workspace.path), branch);
        }
        Ok(())
    }

    /// Remove a workspace's worktree and forget it. The **branch is kept**: commits are never
    /// thrown away here. Without `force`, uncommitted work makes this fail with `worktree_dirty`.
    pub fn delete(&self, workspace_id: &str, force: bool) -> IpcResult<()> {
        let workspace = self.store.workspace(workspace_id)?.ok_or_else(|| {
            IpcError::new("unknown_workspace", "That workspace no longer exists.")
        })?;
        if workspace.kind != "worktree" {
            return Err(IpcError::new(
                "not_deletable",
                "The local workspace cannot be deleted.",
            ));
        }
        let root = self.project_root(&workspace)?;
        let path = PathBuf::from(&workspace.path);

        if path.exists() && root.is_dir() {
            match self.git.worktree_remove(&root, &path, force) {
                Ok(()) => {}
                Err(GitError::Failed { stderr, .. })
                    if !force
                        && (stderr.contains("modified or untracked")
                            || stderr.contains("--force")) =>
                {
                    return Err(IpcError::new(
                        "worktree_dirty",
                        "This workspace has uncommitted changes or untracked files.",
                    ));
                }
                Err(other) => return Err(other.into()),
            }
        } else if root.is_dir() {
            // The folder was deleted behind our back; let git forget it as well.
            let _ = self.git.worktree_prune(&root);
        }
        self.store.remove_worktree(&workspace.id)?;
        Ok(())
    }

    fn project_root(&self, workspace: &WorkspaceRow) -> IpcResult<PathBuf> {
        self.store
            .project(&workspace.project_id)?
            .map(|project| PathBuf::from(project.root_path))
            .ok_or_else(|| IpcError::new("unknown_project", "That project no longer exists."))
    }

    fn undo(&self, root: &Path, path: &Path, branch: &str) {
        let _ = self.git.worktree_remove(root, path, true);
        let _ = self.git.worktree_prune(root);
        let _ = self.git.branch_delete(root, branch);
    }
}

fn branch_for(name: &str) -> String {
    format!("{BRANCH_PREFIX}/{name}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testing::git;
    use crate::git::{normalize, Head};
    use crate::projects::Projects;

    struct Fixture {
        store: Store,
        git: Git,
        _dirs: (tempfile::TempDir, tempfile::TempDir),
        repo: PathBuf,
        worktrees: PathBuf,
        project_id: String,
    }

    impl Fixture {
        fn new() -> Self {
            let (code, worktrees) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
            let (store, git) = (Store::in_memory(), git());
            let added = Projects {
                store: &store,
                git: &git,
            }
            .create("My App", &normalize(code.path()))
            .unwrap();
            Self {
                repo: PathBuf::from(&added.project.root_path),
                worktrees: normalize(worktrees.path()),
                project_id: added.project.id,
                _dirs: (code, worktrees),
                store,
                git,
            }
        }

        fn workspaces(&self) -> Workspaces<'_> {
            Workspaces {
                store: &self.store,
                git: &self.git,
                worktree_root: &self.worktrees,
            }
        }

        fn names(&self) -> Vec<String> {
            self.store
                .workspaces()
                .unwrap()
                .into_iter()
                .map(|w| w.name)
                .collect()
        }
    }

    #[test]
    fn a_workspace_is_a_branch_in_a_worktree_outside_the_repository() {
        let fx = Fixture::new();
        let ws = fx
            .workspaces()
            .create(&fx.project_id, None, "Fix the login bug")
            .unwrap();

        assert_eq!(ws.name, "fix-login-bug");
        assert_eq!(ws.branch.as_deref(), Some("sy/fix-login-bug"));
        let path = PathBuf::from(&ws.path);
        assert_eq!(path, fx.worktrees.join("my-app").join("fix-login-bug"));
        assert!(!path.starts_with(&fx.repo));
        assert_eq!(
            fx.git.head(&path).unwrap(),
            Head::Branch("sy/fix-login-bug".into())
        );
        assert_eq!(fx.names(), ["local", "fix-login-bug"]);
    }

    #[test]
    fn the_same_prompt_twice_gets_a_numbered_name_even_around_leftovers() {
        let fx = Fixture::new();
        fx.workspaces()
            .create(&fx.project_id, None, "add tests")
            .unwrap();
        // A branch Switchyard does not know about still counts as taken.
        fx.git
            .worktree_add(
                &fx.repo,
                &fx.worktrees.join("elsewhere"),
                "sy/add-tests-2",
                &fx.git.default_branch(&fx.repo).unwrap().unwrap(),
            )
            .unwrap();

        let ws = fx
            .workspaces()
            .create(&fx.project_id, None, "Add tests!")
            .unwrap();
        assert_eq!(ws.name, "add-tests-3");
    }

    #[test]
    fn an_empty_prompt_still_gets_a_name() {
        let fx = Fixture::new();
        let ws = fx.workspaces().create(&fx.project_id, None, "   ").unwrap();
        assert!(!ws.name.is_empty());
        assert!(PathBuf::from(&ws.path).is_dir());
    }

    #[test]
    fn it_branches_from_the_base_that_was_asked_for() {
        let fx = Fixture::new();
        fx.git
            .worktree_add(&fx.repo, &fx.worktrees.join("rel"), "release", "HEAD")
            .unwrap();
        std::fs::write(fx.worktrees.join("rel").join("notes.md"), "v1").unwrap();
        let rel = fx.worktrees.join("rel");
        fx.git.run(&rel, &["add", "."]).unwrap();
        fx.git.run(&rel, &["commit", "-qm", "notes"]).unwrap();

        let ws = fx
            .workspaces()
            .create(&fx.project_id, Some("release"), "hotfix")
            .unwrap();
        assert!(
            PathBuf::from(&ws.path).join("notes.md").exists(),
            "started from `release`"
        );

        let err = fx
            .workspaces()
            .create(&fx.project_id, Some("no-such-branch"), "x")
            .unwrap_err();
        assert_eq!(err.code, "git_failed");
        assert_eq!(
            fx.names(),
            ["local", "hotfix"],
            "a failed create leaves nothing behind"
        );
        assert!(!fx.git.branch_exists(&fx.repo, "sy/x").unwrap());
    }

    #[test]
    fn a_repository_without_commits_is_refused_with_an_explanation() {
        let fx = Fixture::new();
        let dir = tempfile::tempdir().unwrap();
        fx.git.init(dir.path()).unwrap();
        let empty = Projects {
            store: &fx.store,
            git: &fx.git,
        }
        .open(dir.path(), false)
        .unwrap();
        let err = fx
            .workspaces()
            .create(&empty.project.id, None, "anything")
            .unwrap_err();
        assert_eq!(err.code, "no_commits");
    }

    #[test]
    fn discarding_removes_folder_branch_and_row() {
        let fx = Fixture::new();
        let ws = fx
            .workspaces()
            .create(&fx.project_id, None, "doomed")
            .unwrap();
        fx.workspaces().discard(&ws).unwrap();
        assert!(!PathBuf::from(&ws.path).exists());
        assert!(!fx.git.branch_exists(&fx.repo, "sy/doomed").unwrap());
        assert_eq!(fx.names(), ["local"]);
    }

    #[test]
    fn deleting_keeps_the_branch_and_protects_uncommitted_work() {
        let fx = Fixture::new();
        let ws = fx.workspaces().create(&fx.project_id, None, "wip").unwrap();
        let path = PathBuf::from(&ws.path);
        std::fs::write(path.join("draft.txt"), "not committed").unwrap();

        let err = fx.workspaces().delete(&ws.id, false).unwrap_err();
        assert_eq!(err.code, "worktree_dirty");
        assert!(path.join("draft.txt").exists(), "nothing was lost");
        assert_eq!(fx.names(), ["local", "wip"]);

        fx.workspaces().delete(&ws.id, true).unwrap();
        assert!(!path.exists());
        assert_eq!(fx.names(), ["local"]);
        assert!(
            fx.git.branch_exists(&fx.repo, "sy/wip").unwrap(),
            "commits are never thrown away"
        );
    }

    #[test]
    fn a_workspace_whose_folder_vanished_can_still_be_deleted() {
        let fx = Fixture::new();
        let ws = fx
            .workspaces()
            .create(&fx.project_id, None, "gone")
            .unwrap();
        std::fs::remove_dir_all(&ws.path).unwrap();
        fx.workspaces().delete(&ws.id, false).unwrap();
        assert_eq!(fx.names(), ["local"]);
        // …and git has forgotten it too, so the name is free again.
        fx.git.branch_delete(&fx.repo, "sy/gone").unwrap();
        assert_eq!(
            fx.workspaces()
                .create(&fx.project_id, None, "gone")
                .unwrap()
                .name,
            "gone"
        );
    }

    #[test]
    fn local_cannot_be_deleted() {
        let fx = Fixture::new();
        let local = fx.store.workspaces().unwrap().remove(0);
        assert_eq!(
            fx.workspaces().delete(&local.id, true).unwrap_err().code,
            "not_deletable"
        );
        assert!(fx.repo.join(".git").exists());
    }
}
