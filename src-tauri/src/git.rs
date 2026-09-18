//! Git, by way of the `git` CLI: the reference implementation, and the only one that respects
//! the user's config, hooks and credentials. Everything goes through [`Git::run`].

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::env::ShellEnv;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("git is not installed, or not on PATH")]
    NotInstalled,
    #[error("`git {command}` failed: {stderr}")]
    Failed { command: String, stderr: String },
    #[error("could not run git: {0}")]
    Io(#[from] std::io::Error),
}

pub type GitResult<T> = Result<T, GitError>;

/// What `HEAD` points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Head {
    Branch(String),
    /// A branch with no commits yet (a freshly initialised repository).
    Unborn(String),
    /// Not on a branch; the abbreviated commit id.
    Detached(String),
}

pub struct Git {
    program: PathBuf,
    env: Vec<(String, String)>,
}

impl Git {
    /// Find git on the *user's* `PATH` and run it with the user's environment.
    pub fn new(env: &ShellEnv) -> GitResult<Self> {
        let cwd = std::env::current_dir().unwrap_or_default();
        let program = env
            .find_program("git", &cwd)
            .ok_or(GitError::NotInstalled)?;
        Ok(Self {
            program,
            env: env
                .vars
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        })
    }

    fn run(&self, cwd: &Path, args: &[&str]) -> GitResult<String> {
        let mut command = Command::new(&self.program);
        command
            .args(args)
            .current_dir(cwd)
            .envs(self.env.iter().map(|(k, v)| (k, v)))
            // Never block on a credential or passphrase prompt nobody can see.
            .env("GIT_TERMINAL_PROMPT", "0")
            // Keep messages in English: a few callers have to recognise them.
            .env("LC_ALL", "C")
            .stdin(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let output = command.output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout)
                .trim_end()
                .to_owned())
        } else {
            Err(GitError::Failed {
                command: args.join(" "),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            })
        }
    }

    /// The top level of the repository containing `path`, or `None` if there isn't one.
    pub fn repo_root(&self, path: &Path) -> GitResult<Option<PathBuf>> {
        match self.run(path, &["rev-parse", "--show-toplevel"]) {
            Ok(root) => Ok(Some(normalize(Path::new(&root)))),
            Err(GitError::Failed { stderr, .. }) if stderr.contains("not a git repository") => {
                Ok(None)
            }
            Err(other) => Err(other),
        }
    }

    pub fn init(&self, path: &Path) -> GitResult<()> {
        self.run(path, &["init"]).map(drop)
    }

    pub fn has_commits(&self, root: &Path) -> GitResult<bool> {
        match self.run(root, &["rev-parse", "--verify", "--quiet", "HEAD"]) {
            Ok(_) => Ok(true),
            Err(GitError::Failed { .. }) => Ok(false),
            Err(other) => Err(other),
        }
    }

    /// Create an empty first commit. A repository without commits cannot have worktrees, so
    /// Switchyard never leaves one it created in that state.
    pub fn initial_commit(&self, root: &Path) -> GitResult<()> {
        self.run(root, &["commit", "--allow-empty", "-m", "Initial commit"])
            .map(drop)
    }

    pub fn head(&self, root: &Path) -> GitResult<Head> {
        match self.run(root, &["symbolic-ref", "--quiet", "--short", "HEAD"]) {
            Ok(branch) if self.has_commits(root)? => Ok(Head::Branch(branch)),
            Ok(branch) => Ok(Head::Unborn(branch)),
            // Not a symbolic ref: HEAD is detached.
            Err(GitError::Failed { .. }) => self
                .run(root, &["rev-parse", "--short", "HEAD"])
                .map(Head::Detached),
            Err(other) => Err(other),
        }
    }
}

/// Canonical form of a path for storing and comparing: symlinks resolved, and on Windows no
/// `\\?\` prefix and no forward slashes (git prints `C:/Users/...`).
pub fn normalize(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    /// A `Git` that ignores the developer's own configuration, with a fixed identity.
    pub fn git() -> Git {
        let mut vars: std::collections::BTreeMap<String, String> = std::env::vars().collect();
        for (key, value) in [
            (
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            ),
            ("GIT_CONFIG_NOSYSTEM", "1"),
            ("GIT_AUTHOR_NAME", "Test"),
            ("GIT_AUTHOR_EMAIL", "test@example.com"),
            ("GIT_COMMITTER_NAME", "Test"),
            ("GIT_COMMITTER_EMAIL", "test@example.com"),
        ] {
            vars.insert(key.into(), value.into());
        }
        Git::new(&ShellEnv {
            vars,
            source: crate::env::EnvSource::Process,
            warning: None,
        })
        .expect("git is required to run the tests")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_directory_is_not_a_repository() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(testing::git().repo_root(dir.path()).unwrap(), None);
    }

    #[test]
    fn the_root_is_found_from_a_subdirectory() {
        let git = testing::git();
        let dir = tempfile::tempdir().unwrap();
        git.init(dir.path()).unwrap();
        let nested = dir.path().join("src").join("deep");
        std::fs::create_dir_all(&nested).unwrap();
        assert_eq!(git.repo_root(&nested).unwrap(), Some(normalize(dir.path())));
    }

    #[test]
    fn head_goes_from_unborn_to_branch_to_detached() {
        let git = testing::git();
        let dir = tempfile::tempdir().unwrap();
        git.init(dir.path()).unwrap();
        git.run(dir.path(), &["checkout", "-b", "trunk"]).unwrap();

        assert_eq!(git.head(dir.path()).unwrap(), Head::Unborn("trunk".into()));
        assert!(!git.has_commits(dir.path()).unwrap());

        git.initial_commit(dir.path()).unwrap();
        assert_eq!(git.head(dir.path()).unwrap(), Head::Branch("trunk".into()));
        assert!(git.has_commits(dir.path()).unwrap());

        git.run(dir.path(), &["checkout", "--detach"]).unwrap();
        assert!(matches!(git.head(dir.path()).unwrap(), Head::Detached(sha) if sha.len() >= 7));
    }

    #[test]
    fn failures_carry_gits_own_words() {
        let dir = tempfile::tempdir().unwrap();
        match testing::git().run(dir.path(), &["log"]) {
            Err(GitError::Failed { command, stderr }) => {
                assert_eq!(command, "log");
                assert!(stderr.contains("not a git repository"), "{stderr}");
            }
            other => panic!("expected a failure, got {other:?}"),
        }
    }
}
