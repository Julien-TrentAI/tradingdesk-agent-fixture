# CLAUDE.md — deskrouter

Guidance for Claude Code (and other coding agents) working in this repository.

## Product

deskrouter is a trading-desk service. Authenticated traders chat with an LLM
order-routing agent that quotes symbols, reads positions/PnL, places and cancels
orders, fetches research, and delegates risk approval to a sub-agent.

## Architecture

- `src/api/` — HTTP entry (`chat`, `admin`). Chat builds and runs the agent.
- `src/agent/` — the agent runtimes: `order_router` (principal), `risk_checker`
  (nested sub-agent), the `action` enum (tool vocabulary), and the `tools`
  registry.
- `src/mcp_server/` — the deskquant MCP server (analyst tools).
- `src/data/` — Postgres client and the Bedrock LLM transport.
- `infra/` — Terraform for AWS resources.

## Conventions

- Rust 2021, `tokio` async. Use `cargo build --locked` / `cargo test --locked`.
- **Account scoping is the most important invariant.** Every data-tier read/
  write MUST be scoped to the caller's verified account, taken from the session,
  never from model-supplied arguments.
- Never interpolate untrusted input into SQL — use parameterized queries.
- Secrets come from the environment / Secrets Manager, never hardcoded.

## Working with agents in this repo

- **MCP servers** (see `.mcp.json`): `deskquant-mcp` (own), `trent`, `github`.
- **Skills** (see `.claude/skills/`): `code-review`, `run-tests`,
  `deploy-to-prod`.
- Before merging a PR, run `code-review` and address High/Critical findings.
