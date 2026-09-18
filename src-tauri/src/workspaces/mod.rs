//! Worktree workspaces: one branch, one folder, one line of work.
//!
//! Everything here is plain git underneath — `git worktree add -b` and `git worktree remove` —
//! so a user can always inspect or undo what Switchyard did with their own tools.

pub mod commands;
mod naming;

use std::path::{Path, PathBuf};

use crate::error::{IpcError, IpcResult};
use crate::git::{normalize, Git, GitError};
use crate::settings::WorkspaceSettings;
use crate::store::{ProjectRow, Store, WorkspaceRow};

pub struct Workspaces<'a> {
    pub store: &'a Store,
    pub git: &'a Git,
    /// Worktrees live in `<root>/<project>/<workspace>`, outside the repositories themselves.
    pub worktree_root: &'a Path,
    /// Branch prefix and friends.
    pub settings: &'a WorkspaceSettings,
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
        let (project, root) = self.usable_project(project_id)?;
        let base = match base {
            Some(base) => base.to_owned(),
            None => self.git.default_branch(&root)?.ok_or_else(|| {
                IpcError::new(
                    "no_base_branch",
                    "Choose a branch to start from: the repository is not on one.",
                )
            })?,
        };

        let project_dir = self.project_dir(&project);
        let seed = self.store.workspaces()?.len();
        // A name is free only if neither its branch nor its folder exists — including leftovers
        // Switchyard does not know about.
        let mut probe_error = None;
        let name = naming::unique(&naming::base_name(prompt, seed), |candidate| {
            let branch_taken = self
                .git
                .branch_exists(&root, &self.settings.branch_for(candidate))
                .unwrap_or_else(|e| {
                    probe_error = Some(e);
                    false
                });
            branch_taken || project_dir.join(candidate).exists()
        });
        if let Some(error) = probe_error {
            return Err(error.into());
        }

        let branch = self.settings.branch_for(&name);
        let path = project_dir.join(&name);
        self.ensure_dir(&project_dir)?;
        self.git.worktree_add(&root, &path, &branch, &base)?;
        self.record(&project, &root, &name, &path, &branch, Some(&base))
    }

    /// Open an *existing* branch as a workspace: a worktree for it, no new branch. This is how a
    /// branch kept by an earlier delete comes back. Git refuses a branch that is already checked
    /// out somewhere, which is exactly the rule we want.
    pub fn open_branch(&self, project_id: &str, branch: &str) -> IpcResult<WorkspaceRow> {
        let (project, root) = self.usable_project(project_id)?;
        if !self.git.branch_exists(&root, branch)? {
            return Err(IpcError::new(
                "unknown_branch",
                format!("There is no branch \"{branch}\"."),
            ));
        }
        let project_dir = self.project_dir(&project);
        // `sy/fix-login` comes back as `fix-login`; other branches are named after themselves.
        let own = self.settings.name_from_branch(branch);
        let base = naming::slugify_name(own).unwrap_or_else(|| naming::base_name("", 0));
        let name = naming::unique(&base, |candidate| project_dir.join(candidate).exists());

        let path = project_dir.join(&name);
        self.ensure_dir(&project_dir)?;
        self.git.worktree_add_existing(&root, &path, branch)?;
        self.record(&project, &root, &name, &path, branch, None)
    }

    fn usable_project(&self, project_id: &str) -> IpcResult<(ProjectRow, PathBuf)> {
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
        Ok((project, root))
    }

    fn project_dir(&self, project: &ProjectRow) -> PathBuf {
        self.worktree_root
            .join(naming::slugify_name(&project.name).unwrap_or_else(|| project.id.clone()))
    }

    fn ensure_dir(&self, dir: &Path) -> IpcResult<()> {
        std::fs::create_dir_all(dir)
            .map_err(|e| IpcError::new("io", format!("Cannot create {}: {e}", dir.display())))
    }

    /// Store a freshly added worktree. `base` is `Some` only when we created the branch.
    fn record(
        &self,
        project: &ProjectRow,
        root: &Path,
        name: &str,
        path: &Path,
        branch: &str,
        base: Option<&str>,
    ) -> IpcResult<WorkspaceRow> {
        // Store the path the way git reports it (symlinks resolved — `/tmp` is `/private/tmp` on
        // macOS), or adoption would later mistake this worktree for an unknown one.
        let stored = normalize(path);
        match self.store.add_worktree(
            &project.id,
            name,
            &stored.to_string_lossy(),
            Some(branch),
            base,
        ) {
            Ok(row) => Ok(row),
            Err(error) => {
                self.undo(root, path, base.is_some().then_some(branch));
                Err(error.into())
            }
        }
    }

    /// Undo a workspace that was created a moment ago and never used, e.g. because its harness
    /// failed to start. A branch we created goes too — it has no commits of its own yet. A
    /// branch that existed before is never touched.
    pub fn discard(&self, workspace: &WorkspaceRow) -> IpcResult<()> {
        let root = self.project_root(workspace)?;
        self.store.remove_worktree(&workspace.id)?;
        let created_branch = workspace
            .base_branch
            .as_ref()
            .and(workspace.branch.as_deref());
        self.undo(&root, Path::new(&workspace.path), created_branch);
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

    /// Take back a worktree, and the branch too if (and only if) we created it.
    fn undo(&self, root: &Path, path: &Path, created_branch: Option<&str>) {
        let _ = self.git.worktree_remove(root, path, true);
        let _ = self.git.worktree_prune(root);
        if let Some(branch) = created_branch {
            let _ = self.git.branch_delete(root, branch);
        }
    }
}

/// Adopt worktrees git knows about but Switchyard does not — made by hand, or orphaned when
/// their project was removed and added again. Returns how many were adopted.
pub fn adopt_unknown(store: &Store, git: &Git, project_id: &str) -> IpcResult<usize> {
    let Some(project) = store.project(project_id)? else {
        return Ok(0);
    };
    let root = PathBuf::from(&project.root_path);
    if !root.is_dir() {
        return Ok(0);
    }
    let known: Vec<PathBuf> = store
        .workspaces()?
        .iter()
        .map(|w| normalize(Path::new(&w.path)))
        .collect();

    let mut adopted = 0;
    for entry in git.worktrees(&root)? {
        if entry.is_main || entry.bare || entry.prunable || known.contains(&entry.path) {
            continue;
        }
        let name = entry
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| entry.path.to_string_lossy().into_owned());
        let recorded = store.add_worktree_if_new(
            &project.id,
            &name,
            &entry.path.to_string_lossy(),
            entry.branch.as_deref(),
        )?;
        adopted += usize::from(recorded.is_some());
    }
    Ok(adopted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testing::git;
    use crate::git::Head;
    use crate::projects::Projects;

    struct Fixture {
        store: Store,
        git: Git,
        _dirs: (tempfile::TempDir, tempfile::TempDir),
        repo: PathBuf,
        worktrees: PathBuf,
        project_id: String,
        settings: WorkspaceSettings,
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
                settings: WorkspaceSettings::default(),
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
                settings: &self.settings,
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
    fn the_branch_prefix_is_a_setting_and_may_be_empty() {
        let mut fx = Fixture::new();
        fx.settings.branch_prefix = "joao/wip".into();
        let ws = fx
            .workspaces()
            .create(&fx.project_id, None, "tidy up")
            .unwrap();
        assert_eq!(ws.branch.as_deref(), Some("joao/wip/tidy-up"));
        assert_eq!(ws.name, "tidy-up");
        fx.workspaces().delete(&ws.id, false).unwrap();
        assert_eq!(
            fx.workspaces()
                .open_branch(&fx.project_id, "joao/wip/tidy-up")
                .unwrap()
                .name,
            "tidy-up"
        );

        fx.settings.branch_prefix = String::new();
        let bare = fx
            .workspaces()
            .create(&fx.project_id, None, "no prefix")
            .unwrap();
        assert_eq!(bare.branch.as_deref(), Some("no-prefix"));
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
    fn a_kept_branch_comes_back_as_a_workspace_with_its_work() {
        let fx = Fixture::new();
        let ws = fx
            .workspaces()
            .create(&fx.project_id, None, "write the docs")
            .unwrap();
        let path = PathBuf::from(&ws.path);
        std::fs::write(path.join("DOCS.md"), "hello").unwrap();
        fx.git.run(&path, &["add", "."]).unwrap();
        fx.git.run(&path, &["commit", "-qm", "docs"]).unwrap();
        fx.workspaces().delete(&ws.id, false).unwrap();
        assert_eq!(fx.names(), ["local"]);

        let back = fx
            .workspaces()
            .open_branch(&fx.project_id, "sy/write-docs")
            .unwrap();

        assert_eq!(back.name, "write-docs");
        assert_eq!(back.branch.as_deref(), Some("sy/write-docs"));
        assert_eq!(back.base_branch, None, "we did not create this branch");
        assert!(
            PathBuf::from(&back.path).join("DOCS.md").exists(),
            "the commit is there"
        );
    }

    #[test]
    fn opening_a_branch_never_puts_the_branch_at_risk() {
        let fx = Fixture::new();
        fx.git.run(&fx.repo, &["branch", "feature/Big Thing"]).ok();
        fx.git.run(&fx.repo, &["branch", "release"]).unwrap();

        let ws = fx
            .workspaces()
            .open_branch(&fx.project_id, "release")
            .unwrap();
        assert_eq!(ws.name, "release");
        // Discarding (the harness failed to start) takes back the folder, not the branch.
        fx.workspaces().discard(&ws).unwrap();
        assert!(!PathBuf::from(&ws.path).exists());
        assert!(fx.git.branch_exists(&fx.repo, "release").unwrap());

        let err = fx
            .workspaces()
            .open_branch(&fx.project_id, "nope")
            .unwrap_err();
        assert_eq!(err.code, "unknown_branch");
    }

    #[test]
    fn a_branch_already_checked_out_is_refused_and_leaves_nothing_behind() {
        let fx = Fixture::new();
        let ws = fx
            .workspaces()
            .create(&fx.project_id, None, "busy")
            .unwrap();
        let err = fx
            .workspaces()
            .open_branch(&fx.project_id, "sy/busy")
            .unwrap_err();
        assert_eq!(err.code, "git_failed");
        assert_eq!(fx.names(), ["local", "busy"]);
        assert!(PathBuf::from(&ws.path).is_dir());
    }

    #[test]
    fn worktrees_switchyard_does_not_know_are_adopted_once() {
        let fx = Fixture::new();
        let ours = fx
            .workspaces()
            .create(&fx.project_id, None, "known")
            .unwrap();
        let by_hand = fx.worktrees.join("made-by-hand");
        fx.git
            .worktree_add(&fx.repo, &by_hand, "experiment", "HEAD")
            .unwrap();
        let stale = fx.worktrees.join("stale");
        fx.git
            .worktree_add(&fx.repo, &stale, "old", "HEAD")
            .unwrap();
        std::fs::remove_dir_all(&stale).unwrap();

        assert_eq!(
            adopt_unknown(&fx.store, &fx.git, &fx.project_id).unwrap(),
            1
        );
        assert_eq!(fx.names(), ["local", "known", "made-by-hand"]);
        let adopted = fx.store.workspaces().unwrap().pop().unwrap();
        assert_eq!(adopted.branch.as_deref(), Some("experiment"));
        assert_eq!(PathBuf::from(&adopted.path), by_hand);
        assert_eq!(adopted.base_branch, None);

        assert_eq!(
            adopt_unknown(&fx.store, &fx.git, &fx.project_id).unwrap(),
            0,
            "idempotent"
        );
        assert!(PathBuf::from(&ours.path).is_dir());
    }

    #[test]
    fn overlapping_adoption_runs_list_a_worktree_once() {
        let fx = Fixture::new();
        fx.git
            .worktree_add(
                &fx.repo,
                &fx.worktrees.join("by-hand"),
                "experiment",
                "HEAD",
            )
            .unwrap();

        // The project list loads from several places at once (startup, window focus), and every
        // load adopts. Run many in parallel, as the app does.
        let adopted: usize = std::thread::scope(|scope| {
            let runs: Vec<_> = (0..8)
                .map(|_| scope.spawn(|| adopt_unknown(&fx.store, &fx.git, &fx.project_id).unwrap()))
                .collect();
            runs.into_iter().map(|run| run.join().unwrap()).sum()
        });

        assert_eq!(adopted, 1);
        assert_eq!(fx.names(), ["local", "by-hand"]);
    }

    #[test]
    fn removing_and_re_adding_a_project_brings_its_workspaces_back() {
        let fx = Fixture::new();
        fx.workspaces()
            .create(&fx.project_id, None, "survivor")
            .unwrap();
        fx.store.remove_project(&fx.project_id).unwrap();

        let again = Projects {
            store: &fx.store,
            git: &fx.git,
        }
        .open(&fx.repo, false)
        .unwrap();
        assert_eq!(
            adopt_unknown(&fx.store, &fx.git, &again.project.id).unwrap(),
            1
        );
        assert_eq!(fx.names(), ["local", "survivor"]);
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
