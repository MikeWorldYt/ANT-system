// Picks the connected AI client that builds the dictionary and holds the command line each one needs.
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::domain::error::R;
use crate::services::ai_clients::{headless_cli, internal, list, AiClient};

// The prompt goes through stdin: a long text with quotes and line breaks cannot pass as an argument of a `.cmd` shim.
const ASK: &str = "Follow the instructions given on standard input.";
const FILE_ASK: &str = "Follow the instructions in the file prompt.md in the current folder and reply with only the dictionary markdown.";
// No MCP servers and no shell or edits: the file names it reads are the user's data and must not be able to steer it into the storage.
const CLAUDE_ARGS: [&str; 13] = ["-p", ASK, "--output-format", "text", "--strict-mcp-config", "--allowedTools", "WebFetch(domain:ant.mikeworld.dev)", "Read", "--disallowedTools", "Bash", "Edit", "Write", "NotebookEdit"];
const CODEX_ARGS: [&str; 7] = ["exec", "--skip-git-repo-check", "--sandbox", "read-only", "--color", "never", "--output-last-message"];
const OPENCODE_PERMISSION: &str = r#"{"edit":"deny","bash":"deny"}"#;
const NONE: &str = "Connect an AI client (Claude Code, Codex or OpenCode) in Connect with your AI Client to build dictionaries with AI.";

pub(super) const PROMPT_FILE: &str = "prompt.md";
pub(super) const ANSWER_FILE: &str = "answer.md";

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Runner {
    Claude,
    Codex,
    OpenCode,
}

// Claude Desktop has no CLI, so it never appears here; the order is the priority.
const ORDER: [(&str, Runner); 3] = [("claudeCode", Runner::Claude), ("codex", Runner::Codex), ("opencode", Runner::OpenCode)];

impl Runner {
    fn id(self) -> &'static str {
        ORDER.iter().find(|(_, r)| *r == self).map_or("", |(id, _)| id)
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Runner::Claude => "Claude Code",
            Runner::Codex => "Codex",
            Runner::OpenCode => "OpenCode",
        }
    }

    // OpenCode is handed the prompt as a file in the work folder, since it is not known to read it from stdin.
    pub(super) fn prompt_in_file(self) -> bool {
        self == Runner::OpenCode
    }

    // Codex also prints progress logs on stdout, so only its last message file is the answer.
    pub(super) fn answer_in_file(self) -> bool {
        self == Runner::Codex
    }

    pub(super) fn args(self, answer: &Path) -> Vec<OsString> {
        match self {
            Runner::Claude => CLAUDE_ARGS.iter().map(OsString::from).collect(),
            Runner::Codex => CODEX_ARGS.iter().map(OsString::from).chain([answer.as_os_str().to_owned(), OsString::from("-")]).collect(),
            Runner::OpenCode => ["run", FILE_ASK].iter().map(OsString::from).collect(),
        }
    }

    pub(super) fn env(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Runner::OpenCode => &[("OPENCODE_PERMISSION", OPENCODE_PERMISSION)],
            _ => &[],
        }
    }

    pub(super) fn failure(self, detail: &str) -> String {
        let lower = detail.to_lowercase();
        if ["login", "log in", "api key", "authenticat", "not logged", "unauthorized", "credentials"].iter().any(|w| lower.contains(w)) {
            let fix = match self {
                Runner::Claude => "run `claude` once and log in",
                Runner::Codex => "run `codex login`",
                Runner::OpenCode => "run `opencode auth login`",
            };
            return format!("{} is not signed in; {fix} ({detail})", self.name());
        }
        format!("{} could not build the dictionary: {detail}", self.name())
    }
}

pub(super) fn candidates(clients: &[AiClient]) -> Vec<Runner> {
    ORDER.iter().filter(|(id, _)| clients.iter().any(|c| c.id == *id && c.connected)).map(|(_, r)| *r).collect()
}

// A client connected without its CLI on PATH (Codex or OpenCode desktop apps) is skipped for the next one.
pub(super) fn choose() -> R<(Runner, PathBuf)> {
    let found = candidates(&list()?);
    let first = found.first().ok_or_else(|| internal(NONE))?;
    found.iter().find_map(|r| headless_cli(r.id()).map(|cli| (*r, cli))).ok_or_else(|| internal(format!("{} is connected but its command-line tool was not found on PATH", first.name())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(id: &'static str, connected: bool) -> AiClient {
        AiClient { id, name: id, installed: true, connected, config_path: None, restart_needed: false }
    }

    #[test]
    fn dictionary_ai_picks_nothing_without_a_connected_cli_client() {
        assert!(candidates(&[]).is_empty());
        assert!(candidates(&[client("claudeDesktop", true), client("claudeCode", false), client("codex", false), client("opencode", false)]).is_empty());
    }

    #[test]
    fn dictionary_ai_prefers_claude_code_then_codex_then_opencode() {
        let all = [client("opencode", true), client("codex", true), client("claudeDesktop", true), client("claudeCode", true)];
        assert_eq!(candidates(&all), [Runner::Claude, Runner::Codex, Runner::OpenCode]);
        assert_eq!(candidates(&all[..2]), [Runner::Codex, Runner::OpenCode]);
        assert_eq!(candidates(&[client("opencode", true), client("codex", false)]), [Runner::OpenCode]);
    }

    #[test]
    fn dictionary_ai_gives_each_client_its_own_command_line() {
        let answer = Path::new("out.md");
        assert_eq!(Runner::Claude.args(answer)[0], "-p");
        let codex = Runner::Codex.args(answer);
        assert_eq!((codex[0].to_str(), codex[codex.len() - 2].to_str(), codex[codex.len() - 1].to_str()), (Some("exec"), Some("out.md"), Some("-")));
        assert_eq!(Runner::OpenCode.args(answer)[0], "run");
        assert!(Runner::OpenCode.prompt_in_file() && !Runner::Claude.prompt_in_file() && !Runner::Codex.prompt_in_file());
        assert!(Runner::Codex.answer_in_file() && !Runner::Claude.answer_in_file());
    }

    #[test]
    fn dictionary_ai_hints_the_sign_in_command_of_the_client() {
        assert!(Runner::Codex.failure("Not logged in").contains("`codex login`"));
        assert!(Runner::OpenCode.failure("no credentials").contains("`opencode auth login`"));
        assert!(Runner::Claude.failure("Please log in").contains("`claude`"));
        assert_eq!(Runner::Codex.failure("boom"), "Codex could not build the dictionary: boom");
    }
}
