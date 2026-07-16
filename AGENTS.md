# AGENTS.md — deskrouter

Instruction file for coding agents (Codex, Cursor, aider, etc.) operating in
this repo. Mirrors `CLAUDE.md`.

- Build: `cargo build --locked`. Test: `cargo test --locked`.
- Preserve account scoping (caller's verified account, never model-supplied).
- Parameterized SQL only; no hardcoded secrets.
- The agent runtimes live in `src/agent/`; the tool vocabulary is the
  `AgentAction` enum in `src/agent/action.rs`.
