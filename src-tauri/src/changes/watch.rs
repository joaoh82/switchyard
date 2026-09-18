//! Tell the UI when a workspace's files change, without being told about `node_modules`.
//!
//! One workspace is watched at a time — the one on screen. Events are not interpreted: any
//! change, after things go quiet, produces one "something changed" signal, and the UI asks git
//! again. Git is the judge of what changed; the watcher only says *when* to ask.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::WalkBuilder;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Wait this long after the last event before signalling…
const QUIET: Duration = Duration::from_millis(250);
/// …but never hold a signal back longer than this while events keep coming.
const MAX_WAIT: Duration = Duration::from_millis(1500);

/// Watches until dropped.
pub struct WorkspaceWatcher {
    /// The debounce thread holds this weakly: dropping us drops the watcher, which closes the
    /// event channel, which ends the thread.
    _watcher: Arc<Mutex<RecommendedWatcher>>,
}

impl WorkspaceWatcher {
    /// Watch `root`, plus `git_dir` (where `HEAD` and the index live — for a worktree that is
    /// *outside* `root`), calling `on_change` after each burst of activity.
    pub fn start(
        root: &Path,
        git_dir: Option<&Path>,
        on_change: impl Fn() + Send + 'static,
    ) -> notify::Result<Self> {
        let (tx, rx) = mpsc::channel::<Event>();
        let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
            if let Ok(event) = event {
                let _ = tx.send(event);
            }
        })?;

        if cfg!(target_os = "linux") {
            // inotify watches single directories, and a recursive watch would descend into
            // every ignored folder too. Watch exactly the directories git cares about.
            for dir in unignored_dirs(root) {
                let _ = watcher.watch(&dir, RecursiveMode::NonRecursive);
            }
        } else {
            // FSEvents and ReadDirectoryChangesW watch a whole tree for the price of one.
            watcher.watch(root, RecursiveMode::Recursive)?;
        }
        if let Some(git_dir) = git_dir {
            // Commits, checkouts and staging happen here.
            let _ = watcher.watch(git_dir, RecursiveMode::NonRecursive);
        }

        let watcher = Arc::new(Mutex::new(watcher));
        let weak = Arc::downgrade(&watcher);
        let root = root.to_path_buf();
        let git_dir = git_dir.map(Path::to_path_buf);
        std::thread::Builder::new()
            .name("workspace-watch".into())
            .spawn(move || debounce(&rx, &root, git_dir.as_deref(), &weak, &on_change))?;
        Ok(Self { _watcher: watcher })
    }
}

/// Collapse bursts of events into single signals. Ends when the watcher is dropped.
fn debounce(
    rx: &mpsc::Receiver<Event>,
    root: &Path,
    git_dir: Option<&Path>,
    watcher: &Weak<Mutex<RecommendedWatcher>>,
    on_change: &dyn Fn(),
) {
    let ignore = ignore_rules(root);
    let relevant = |event: &Event| {
        if matches!(event.kind, EventKind::Access(_)) {
            return false;
        }
        let relevant = event
            .paths
            .iter()
            .any(|path| is_relevant(path, root, git_dir, &ignore));
        if relevant && cfg!(target_os = "linux") && matches!(event.kind, EventKind::Create(_)) {
            // Per-directory watches do not cover folders that appear later; add them.
            if let Some(watcher) = watcher.upgrade() {
                let mut watcher = watcher.lock().unwrap_or_else(|e| e.into_inner());
                for created in event.paths.iter().filter(|path| path.is_dir()) {
                    for dir in unignored_dirs(created) {
                        let _ = watcher.watch(&dir, RecursiveMode::NonRecursive);
                    }
                }
            }
        }
        relevant
    };
    loop {
        // Sleep until something relevant happens.
        match rx.recv() {
            Ok(event) if relevant(&event) => {}
            Ok(_) => continue,
            Err(_) => return,
        }
        let started = Instant::now();
        loop {
            let wait = QUIET.min(MAX_WAIT.saturating_sub(started.elapsed()));
            match rx.recv_timeout(wait) {
                Ok(event) => {
                    relevant(&event); // keeps watching folders created mid-burst
                    if started.elapsed() >= MAX_WAIT {
                        break;
                    }
                }
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
        on_change();
    }
}

fn is_relevant(path: &Path, root: &Path, git_dir: Option<&Path>, ignore: &Gitignore) -> bool {
    if let Some(git_dir) = git_dir {
        if path.starts_with(git_dir) {
            // Only what changes the answer to "what changed?"; not lock files or object writes.
            return matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("HEAD" | "index" | "ORIG_HEAD" | "MERGE_HEAD")
            );
        }
    }
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    if relative.components().any(|part| part.as_os_str() == ".git") {
        return false;
    }
    !ignore
        .matched_path_or_any_parents(relative, path.is_dir())
        .is_ignore()
}

/// The root `.gitignore` and `.git/info/exclude`. Nested ignore files are not consulted here:
/// an event they would have excluded merely causes one cheap, harmless refresh.
fn ignore_rules(root: &Path) -> Gitignore {
    let mut builder = GitignoreBuilder::new(root);
    let _ = builder.add(root.join(".gitignore"));
    let _ = builder.add(root.join(".git").join("info").join("exclude"));
    builder.build().unwrap_or_else(|_| Gitignore::empty())
}

/// Every directory under `root` that git does not ignore, `root` included.
fn unignored_dirs(root: &Path) -> Vec<PathBuf> {
    WalkBuilder::new(root)
        .hidden(false)
        .require_git(false)
        .filter_entry(|entry| entry.file_name() != ".git")
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_dir()))
        .map(|entry| entry.into_path())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::create_dir_all(dir.path().join("node_modules/pkg")).unwrap();
        std::fs::write(dir.path().join(".gitignore"), "node_modules/\n*.log\n").unwrap();
        dir
    }

    fn watch(dir: &tempfile::TempDir) -> (WorkspaceWatcher, Arc<AtomicUsize>) {
        let signals = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&signals);
        let root = dunce::canonicalize(dir.path()).unwrap();
        let watcher = WorkspaceWatcher::start(&root, Some(&root.join(".git")), move || {
            counter.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();
        // Let the OS arm the watches — and let the fixture's own writes drain. macOS's FSEvents
        // happily reports changes from just *before* a watch began, so without this the repo
        // being set up can show up as a signal in the test that asserts there are none.
        std::thread::sleep(QUIET * 3);
        signals.store(0, Ordering::SeqCst);
        (watcher, signals)
    }

    fn wait_for(signals: &AtomicUsize, at_least: usize) -> bool {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if signals.load(Ordering::SeqCst) >= at_least {
                return true;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        false
    }

    #[test]
    fn a_burst_of_edits_becomes_one_signal() {
        let dir = repo();
        let (_watcher, signals) = watch(&dir);
        for n in 0..20 {
            std::fs::write(dir.path().join("src").join(format!("file{n}.rs")), "x").unwrap();
        }
        assert!(wait_for(&signals, 1), "edits were not noticed");
        std::thread::sleep(QUIET * 3);
        assert_eq!(
            signals.load(Ordering::SeqCst),
            1,
            "twenty writes, one refresh"
        );
    }

    #[test]
    fn ignored_files_and_git_internals_stay_silent_but_commits_do_not() {
        let dir = repo();
        let (_watcher, signals) = watch(&dir);
        std::fs::write(dir.path().join("node_modules/pkg/index.js"), "x").unwrap();
        std::fs::write(dir.path().join("debug.log"), "x").unwrap();
        std::fs::write(dir.path().join(".git/index.lock"), "x").unwrap();
        std::thread::sleep(MAX_WAIT);
        assert_eq!(signals.load(Ordering::SeqCst), 0);

        std::fs::write(dir.path().join(".git/index"), "staged something").unwrap();
        assert!(wait_for(&signals, 1), "a change to the index is a change");
    }

    #[test]
    fn folders_created_later_are_watched_too() {
        let dir = repo();
        let (_watcher, signals) = watch(&dir);
        std::fs::create_dir_all(dir.path().join("src/brand/new")).unwrap();
        assert!(wait_for(&signals, 1), "the new folder was not noticed");
        std::thread::sleep(QUIET * 2);

        std::fs::write(dir.path().join("src/brand/new/file.rs"), "x").unwrap();
        assert!(
            wait_for(&signals, 2),
            "a file in a folder created after start went unnoticed"
        );
    }

    #[test]
    fn dropping_the_watcher_stops_the_signals() {
        let dir = repo();
        let (watcher, signals) = watch(&dir);
        drop(watcher);
        std::fs::write(dir.path().join("src/after.rs"), "x").unwrap();
        std::thread::sleep(MAX_WAIT);
        assert_eq!(signals.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn relevance_follows_gitignore() {
        let dir = repo();
        let root = dir.path();
        let rules = ignore_rules(root);
        let git = root.join(".git");
        let check = |path: &str| is_relevant(&root.join(path), root, Some(&git), &rules);
        assert!(check("src/main.rs") && check("README.md") && check(".git/HEAD"));
        assert!(!check("node_modules/pkg/index.js") && !check("app.log"));
        assert!(!check(".git/objects/ab/cdef") && !check(".git/index.lock"));
        assert!(!is_relevant(
            Path::new("/somewhere/else"),
            root,
            Some(&git),
            &rules
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_watches_only_directories_git_cares_about() {
        let dir = repo();
        let dirs = unignored_dirs(dir.path());
        assert!(dirs.iter().any(|d| d.ends_with("src")));
        assert!(!dirs
            .iter()
            .any(|d| d.to_string_lossy().contains("node_modules")));
        assert!(!dirs.iter().any(|d| d.to_string_lossy().contains(".git")));
    }
}
