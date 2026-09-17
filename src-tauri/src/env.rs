//! The environment harnesses run in.
//!
//! A GUI app does not inherit the user's shell environment: launched from a dock or desktop
//! launcher, `PATH` lacks everything that `.zshrc`, mise, nvm, cargo or Homebrew add — so
//! `claude` "does not exist" even though it works in every terminal. We fix that the way editors
//! do: run the user's login shell once, ask it for its environment, and use that for every spawn.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;
use specta::Type;

/// Makes this executable print its environment and exit. See [`print_env_and_exit_if_asked`].
const PRINT_ENV_FLAG: &str = "--switchyard-print-env";
const BEGIN: &[u8] = b"\0SWITCHYARD-ENV-BEGIN\0";
const END: &[u8] = b"\0SWITCHYARD-ENV-END\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum EnvSource {
    /// Captured from the user's login shell.
    LoginShell,
    /// This process's own environment: always on Windows, and the fallback elsewhere.
    Process,
}

#[derive(Debug, Clone)]
pub struct ShellEnv {
    pub vars: BTreeMap<String, String>,
    pub source: EnvSource,
    /// Why the login shell could not be used, when it could not.
    pub warning: Option<String>,
}

impl ShellEnv {
    pub fn resolve() -> Self {
        match platform::from_login_shell() {
            Ok(Some(vars)) => Self {
                vars,
                source: EnvSource::LoginShell,
                warning: None,
            },
            Ok(None) => Self::from_process(None),
            Err(reason) => Self::from_process(Some(reason)),
        }
    }

    fn from_process(warning: Option<String>) -> Self {
        Self {
            vars: std::env::vars().collect(),
            source: EnvSource::Process,
            warning,
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        lookup(&self.vars, key, cfg!(windows))
    }

    /// Locate `program` the way a shell would, using *this* environment's `PATH`.
    pub fn find_program(&self, program: &str, cwd: &std::path::Path) -> Option<PathBuf> {
        which::which_in(program, self.get("PATH"), cwd).ok()
    }

    /// The user's interactive shell and the arguments to start it with.
    pub fn default_shell(&self) -> (String, Vec<String>) {
        platform::default_shell(self)
    }

    pub fn home_dir(&self) -> Option<PathBuf> {
        self.get(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .map(PathBuf::from)
    }
}

/// Windows variable names are case-insensitive, and the ones that matter are not spelled the way
/// everyone writes them: it is `Path` and `ComSpec` there, not `PATH` and `COMSPEC`.
fn lookup<'a>(vars: &'a BTreeMap<String, String>, key: &str, ignore_case: bool) -> Option<&'a str> {
    vars.get(key)
        .or_else(|| {
            ignore_case
                .then(|| vars.iter().find(|(name, _)| name.eq_ignore_ascii_case(key)))
                .flatten()
                .map(|(_, value)| value)
        })
        .map(String::as_str)
}

/// Call first thing in `main`. When the login shell runs us with [`PRINT_ENV_FLAG`], dump the
/// environment it gave us and exit. Using our own binary avoids depending on `env -0` (absent on
/// some systems) or on any particular shell's syntax.
pub fn print_env_and_exit_if_asked() {
    if std::env::args().nth(1).as_deref() != Some(PRINT_ENV_FLAG) {
        return;
    }
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(BEGIN);
    for (key, value) in std::env::vars_os() {
        let _ = out.write_all(key.as_encoded_bytes());
        let _ = out.write_all(b"=");
        let _ = out.write_all(value.as_encoded_bytes());
        let _ = out.write_all(b"\0");
    }
    let _ = out.write_all(END);
    let _ = out.flush();
    std::process::exit(0);
}

/// Extract the variables printed between the markers, ignoring whatever the shell's startup
/// files printed around them.
#[cfg(any(unix, test))]
fn parse_dump(output: &[u8]) -> Option<BTreeMap<String, String>> {
    let start = find(output, BEGIN)? + BEGIN.len();
    let end = start + find(&output[start..], END)?;
    let vars: BTreeMap<_, _> = output[start..end]
        .split(|&b| b == 0)
        .filter_map(|entry| {
            let entry = std::str::from_utf8(entry).ok()?;
            let (key, value) = entry.split_once('=')?;
            // Facts about the throwaway shell, not about the user's environment.
            let transient = matches!(key, "_" | "SHLVL" | "PWD" | "OLDPWD");
            (!key.is_empty() && !transient).then(|| (key.to_owned(), value.to_owned()))
        })
        .collect();
    (!vars.is_empty()).then_some(vars)
}

#[cfg(any(unix, test))]
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(unix)]
mod platform {
    use std::collections::BTreeMap;
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    use super::{parse_dump, ShellEnv, PRINT_ENV_FLAG};

    /// Slow shell startup files are common; a hung one must not hang the app.
    const TIMEOUT: Duration = Duration::from_secs(8);

    pub(super) fn from_login_shell() -> Result<Option<BTreeMap<String, String>>, String> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let exe =
            std::env::current_exe().map_err(|e| format!("cannot locate own executable: {e}"))?;
        let exe = exe.to_str().ok_or("own executable path is not UTF-8")?;
        // Single-quote the path; this quoting is understood by sh, bash, zsh and fish alike.
        let script = format!("'{}' {PRINT_ENV_FLAG}", exe.replace('\'', r"'\''"));

        let mut child = Command::new(&shell)
            // Interactive + login, so both profile and rc files run — version managers hook
            // into either.
            .args(["-i", "-l", "-c", &script])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("cannot run {shell}: {e}"))?;

        let mut stdout = child.stdout.take().expect("stdout was piped");
        let reader = std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stdout.read_to_end(&mut buf);
            buf
        });

        let started = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if started.elapsed() > TIMEOUT => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{shell} did not finish within {TIMEOUT:?}"));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(15)),
                Err(e) => return Err(format!("waiting for {shell} failed: {e}")),
            }
        }

        let output = reader.join().map_err(|_| "reading shell output failed")?;
        parse_dump(&output)
            .map(Some)
            .ok_or_else(|| format!("{shell} produced no environment"))
    }

    pub(super) fn default_shell(env: &ShellEnv) -> (String, Vec<String>) {
        let shell = env.get("SHELL").unwrap_or("/bin/sh").to_owned();
        // macOS terminals start login shells by convention; Linux ones do not.
        let args = if cfg!(target_os = "macos") {
            vec!["-l".to_owned()]
        } else {
            vec![]
        };
        (shell, args)
    }
}

#[cfg(windows)]
mod platform {
    use std::collections::BTreeMap;

    use super::ShellEnv;

    /// Windows GUI apps get the full user environment already.
    pub(super) fn from_login_shell() -> Result<Option<BTreeMap<String, String>>, String> {
        Ok(None)
    }

    pub(super) fn default_shell(env: &ShellEnv) -> (String, Vec<String>) {
        let cwd = std::env::current_dir().unwrap_or_default();
        for candidate in ["pwsh.exe", "powershell.exe"] {
            if env.find_program(candidate, &cwd).is_some() {
                return (candidate.to_owned(), vec!["-NoLogo".to_owned()]);
            }
        }
        (env.get("COMSPEC").unwrap_or("cmd.exe").to_owned(), vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_variables_between_markers_and_ignores_shell_noise() {
        let mut dump = b"motd from .zshrc\n".to_vec();
        dump.extend_from_slice(BEGIN);
        dump.extend_from_slice(
            b"PATH=/a:/b\0EMPTY=\0MULTI=line1\nline2\0WITH_EQ=a=b\0SHLVL=3\0_=/x\0",
        );
        dump.extend_from_slice(END);
        dump.extend_from_slice(b"\nlogout\n");

        let vars = parse_dump(&dump).unwrap();

        assert_eq!(vars["PATH"], "/a:/b");
        assert_eq!(vars["EMPTY"], "");
        assert_eq!(vars["MULTI"], "line1\nline2");
        assert_eq!(vars["WITH_EQ"], "a=b");
        assert!(!vars.contains_key("SHLVL") && !vars.contains_key("_"));
    }

    #[test]
    fn rejects_output_without_a_complete_dump() {
        assert!(parse_dump(b"command not found").is_none());
        let mut truncated = BEGIN.to_vec();
        truncated.extend_from_slice(b"PATH=/a\0");
        assert!(parse_dump(&truncated).is_none());
    }

    #[test]
    fn windows_style_lookups_ignore_case_but_prefer_an_exact_match() {
        let vars: BTreeMap<String, String> = [("Path", "C:\\Windows"), ("ComSpec", "cmd.exe")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        assert_eq!(lookup(&vars, "PATH", true), Some("C:\\Windows"));
        assert_eq!(lookup(&vars, "COMSPEC", true), Some("cmd.exe"));
        assert_eq!(
            lookup(&vars, "PATH", false),
            None,
            "Unix names are case-sensitive"
        );
        assert_eq!(lookup(&vars, "Path", false), Some("C:\\Windows"));
    }

    #[test]
    fn process_fallback_carries_the_reason() {
        let env = ShellEnv::from_process(Some("shell timed out".into()));
        assert_eq!(env.source, EnvSource::Process);
        assert_eq!(env.warning.as_deref(), Some("shell timed out"));
        assert!(!env.vars.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn finds_programs_on_this_environments_path_only() {
        let mut env = ShellEnv::from_process(None);
        let cwd = std::env::current_dir().unwrap();
        assert!(env.find_program("sh", &cwd).is_some());
        env.vars.insert("PATH".into(), "/nonexistent".into());
        assert!(env.find_program("sh", &cwd).is_none());
    }
}
