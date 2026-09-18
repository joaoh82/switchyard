//! Harnesses: terminal coding agents, described as pure configuration.
//!
//! A definition is a command plus argument templates for the few things Switchyard needs to do
//! with an agent. Adding one must never need code — M4 makes these user-editable; until then the
//! built-ins below are all there is. See `docs/04-harnesses.md`.

use serde::Serialize;
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PromptTransport {
    /// The prompt is an argument. Simple and reliable.
    Argv,
    /// The prompt is typed into the terminal once the harness is up. Arrives with M4; no
    /// built-in needs it.
    #[allow(dead_code)]
    Stdin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SessionIdMode {
    /// We choose the harness's session id up front, so resume is deterministic.
    Assigned,
    /// The harness picks; we resume "the latest session in this directory", which is safe
    /// because every workspace has a directory of its own.
    LatestInCwd,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HarnessDef {
    /// Stable key, recorded with sessions.
    pub id: String,
    pub label: String,
    pub command: String,
    /// Always passed.
    pub base_args: Vec<String>,
    pub model_args: Vec<String>,
    pub effort_args: Vec<String>,
    pub session_args: Vec<String>,
    pub prompt_args: Vec<String>,
    pub resume_args: Vec<String>,
    pub fork_args: Vec<String>,
    /// Effort levels to offer. Empty hides the picker.
    pub efforts: Vec<String>,
    /// Model suggestions. Free text is always accepted: names change faster than we ship.
    pub models: Vec<String>,
    pub prompt_transport: PromptTransport,
    pub session_id_mode: SessionIdMode,
}

/// Values for the placeholders. `None` means "not given": the arg group using it is dropped.
#[derive(Debug, Clone, Default)]
pub struct LaunchValues {
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub session_id: Option<String>,
}

impl HarnessDef {
    /// The argv (without the command) for starting a new session.
    pub fn start_args(&self, values: &LaunchValues) -> Vec<String> {
        [
            &self.base_args,
            &self.model_args,
            &self.effort_args,
            &self.session_args,
            &self.prompt_args,
        ]
        .into_iter()
        .flat_map(|group| expand(group, values))
        .collect()
    }
}

/// Substitute placeholders in one arg group. Each element stays exactly one argument whatever
/// the value contains — spaces, quotes, newlines — because nothing is ever re-split and no shell
/// is involved. A group that needs a value nobody gave is dropped whole, so choosing no model
/// leaves no dangling `--model`.
fn expand(group: &[String], values: &LaunchValues) -> Vec<String> {
    let lookup = |name: &str| match name {
        "prompt" => values.prompt.as_deref(),
        "model" => values.model.as_deref(),
        "effort" => values.effort.as_deref(),
        "session_id" => values.session_id.as_deref(),
        _ => None,
    };
    let mut out = Vec::with_capacity(group.len());
    for template in group {
        let mut arg = String::new();
        let mut rest = template.as_str();
        while let Some(open) = rest.find('{') {
            let Some(close) = rest[open..].find('}') else {
                break;
            };
            let name = &rest[open + 1..open + close];
            if !is_placeholder(name) {
                // Literal braces, e.g. JSON in an argument: copy through untouched.
                arg.push_str(&rest[..=open + close]);
            } else if let Some(value) = lookup(name).filter(|v| !v.is_empty()) {
                arg.push_str(&rest[..open]);
                arg.push_str(value);
            } else {
                return Vec::new();
            }
            rest = &rest[open + close + 1..];
        }
        arg.push_str(rest);
        out.push(arg);
    }
    out
}

fn is_placeholder(name: &str) -> bool {
    matches!(name, "prompt" | "model" | "effort" | "session_id")
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

/// The harnesses Switchyard knows out of the box. Flags verified against each CLI's `--help`
/// (versions in `docs/04-harnesses.md`); they move, so M4 lets users correct them.
pub fn builtin() -> Vec<HarnessDef> {
    vec![
        HarnessDef {
            id: "claude".into(),
            label: "Claude Code".into(),
            command: "claude".into(),
            base_args: vec![],
            model_args: strings(&["--model", "{model}"]),
            effort_args: strings(&["--effort", "{effort}"]),
            session_args: strings(&["--session-id", "{session_id}"]),
            prompt_args: strings(&["{prompt}"]),
            resume_args: strings(&["--resume", "{session_id}"]),
            fork_args: strings(&["--resume", "{session_id}", "--fork-session"]),
            efforts: strings(&["low", "medium", "high", "xhigh", "max"]),
            models: strings(&["fable", "opus", "sonnet", "haiku"]),
            prompt_transport: PromptTransport::Argv,
            session_id_mode: SessionIdMode::Assigned,
        },
        HarnessDef {
            id: "codex".into(),
            label: "Codex".into(),
            command: "codex".into(),
            base_args: vec![],
            model_args: strings(&["-m", "{model}"]),
            effort_args: strings(&["-c", "model_reasoning_effort=\"{effort}\""]),
            session_args: vec![],
            prompt_args: strings(&["{prompt}"]),
            resume_args: strings(&["resume", "--last"]),
            fork_args: strings(&["fork", "--last"]),
            efforts: strings(&["low", "medium", "high"]),
            models: vec![],
            prompt_transport: PromptTransport::Argv,
            session_id_mode: SessionIdMode::LatestInCwd,
        },
        HarnessDef {
            id: "grok".into(),
            label: "Grok".into(),
            command: "grok".into(),
            base_args: vec![],
            model_args: strings(&["-m", "{model}"]),
            effort_args: strings(&["--reasoning-effort", "{effort}"]),
            session_args: strings(&["--session-id", "{session_id}"]),
            prompt_args: strings(&["{prompt}"]),
            resume_args: strings(&["--resume", "{session_id}"]),
            fork_args: strings(&["--resume", "{session_id}", "--fork-session"]),
            efforts: strings(&["low", "medium", "high"]),
            models: vec![],
            prompt_transport: PromptTransport::Argv,
            session_id_mode: SessionIdMode::Assigned,
        },
        HarnessDef {
            id: "opencode".into(),
            label: "OpenCode".into(),
            command: "opencode".into(),
            base_args: vec![],
            model_args: strings(&["-m", "{model}"]),
            effort_args: vec![],
            session_args: vec![],
            prompt_args: strings(&["--prompt", "{prompt}"]),
            resume_args: strings(&["--continue"]),
            fork_args: strings(&["--continue", "--fork"]),
            efforts: vec![],
            models: vec![],
            prompt_transport: PromptTransport::Argv,
            session_id_mode: SessionIdMode::LatestInCwd,
        },
    ]
}

pub fn find(id: &str) -> Option<HarnessDef> {
    builtin().into_iter().find(|harness| harness.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(prompt: &str, model: &str, effort: &str) -> LaunchValues {
        let some = |s: &str| (!s.is_empty()).then(|| s.to_owned());
        LaunchValues {
            prompt: some(prompt),
            model: some(model),
            effort: some(effort),
            session_id: Some("11111111-2222-3333-4444-555555555555".into()),
        }
    }

    #[test]
    fn claude_gets_model_effort_session_and_prompt() {
        let args = find("claude")
            .unwrap()
            .start_args(&values("fix the bug", "opus", "high"));
        assert_eq!(
            args,
            [
                "--model",
                "opus",
                "--effort",
                "high",
                "--session-id",
                "11111111-2222-3333-4444-555555555555",
                "fix the bug",
            ]
        );
    }

    #[test]
    fn groups_without_a_value_vanish_whole() {
        let args = find("claude").unwrap().start_args(&values("", "", ""));
        assert_eq!(
            args,
            ["--session-id", "11111111-2222-3333-4444-555555555555"]
        );
        assert!(find("opencode")
            .unwrap()
            .start_args(&values("", "", "high"))
            .is_empty());
    }

    #[test]
    fn a_prompt_is_always_exactly_one_argument() {
        let nasty = "rename `foo` to \"bar\"; rm -rf $HOME\n--model evil 'quoted' {model}";
        let args = find("opencode").unwrap().start_args(&values(nasty, "", ""));
        assert_eq!(args, ["--prompt", nasty]);
    }

    #[test]
    fn placeholders_substitute_inside_an_argument_and_other_braces_are_literal() {
        let args = find("codex")
            .unwrap()
            .start_args(&values("go", "", "medium"));
        assert_eq!(args, ["-c", "model_reasoning_effort=\"medium\"", "go"]);

        let group = strings(&["--config", "{\"a\":{\"b\":1}}", "--tag={effort}"]);
        assert_eq!(
            expand(&group, &values("", "", "low")),
            ["--config", "{\"a\":{\"b\":1}}", "--tag=low"]
        );
    }

    #[test]
    fn builtin_ids_are_unique_and_lookups_work() {
        let all = builtin();
        let mut ids: Vec<_> = all.iter().map(|h| h.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all.len());
        assert!(find("nope").is_none());
    }
}
