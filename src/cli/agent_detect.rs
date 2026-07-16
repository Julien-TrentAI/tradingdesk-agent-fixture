//! CLI coding-agent DETECTION.
//!
//! FALSE-POSITIVE TRAP (intentional). `CliAgentKind` enumerates external CLI
//! coding agents this tool can *recognize* in a developer's environment (by
//! probing for their config files) so it can print a compatibility note. The
//! variants are agent NAMES, and this file matches on them — but nothing here
//! is a runtime: there is no LLM call, no tool loop, no agency. This is a
//! detector, not an agent. A scan must NOT mint one inventory `agent` per
//! variant.

use std::path::Path;

/// Coding-agent CLIs this environment probe can recognize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliAgentKind {
    ClaudeCode,
    Cursor,
    Codex,
    Aider,
    Copilot,
    GeminiCli,
    Windsurf,
    Unknown,
}

impl CliAgentKind {
    /// Detect which coding-agent CLI (if any) is configured in `root`.
    /// Pure filesystem probing — dispatched via `match` on marker files.
    pub fn detect(root: &Path) -> Self {
        let has = |p: &str| root.join(p).exists();
        match () {
            _ if has(".claude") => CliAgentKind::ClaudeCode,
            _ if has(".cursor") => CliAgentKind::Cursor,
            _ if has(".codex") => CliAgentKind::Codex,
            _ if has(".aider.conf.yml") => CliAgentKind::Aider,
            _ if has(".github/copilot-instructions.md") => CliAgentKind::Copilot,
            _ if has(".gemini") => CliAgentKind::GeminiCli,
            _ if has(".windsurfrules") => CliAgentKind::Windsurf,
            _ => CliAgentKind::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            CliAgentKind::ClaudeCode => "Claude Code",
            CliAgentKind::Cursor => "Cursor",
            CliAgentKind::Codex => "OpenAI Codex",
            CliAgentKind::Aider => "aider",
            CliAgentKind::Copilot => "GitHub Copilot",
            CliAgentKind::GeminiCli => "Gemini CLI",
            CliAgentKind::Windsurf => "Windsurf",
            CliAgentKind::Unknown => "unknown",
        }
    }
}
