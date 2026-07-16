# deskrouter — trading-desk order-routing agent (security validation fixture)

> ## ⚠️ DELIBERATELY VULNERABLE — DO NOT USE
> This repository is a **synthetic security-scanner validation fixture** written
> in Rust. It contains intentionally planted vulnerabilities (SQL injection,
> cross-account IDOR, SSRF, RCE, public S3 bucket, over-broad IAM, committed
> credentials, prompt injection, insecure CI/CD coding-agent workflows) and only
> fake/placeholder secrets and fake AWS accounts. **Do not deploy it, do not
> copy any code or workflow from it, and do not treat any pattern here as
> guidance.** It exists only to be scanned.

## What this is

**deskrouter** is a fictional trading-desk service where an authenticated trader
chats with an LLM **order-routing agent** that can quote symbols, look up
positions and PnL, place/cancel orders, fetch research, and delegate to a
risk-check sub-agent. It is a realistic stand-in for a Rust agentic application
and is seeded to validate **all** of Trent's analyses at once:

1. **Threat model** — a conventional Rust web/data service on AWS carrying
   classical threats (SQLi, IDOR, SSRF, RCE, IAM, encryption, secrets).
2. **Agentic inventory posture** — the desk agent is expressed with idiomatic
   Rust: a `Lazy<HashMap>` **tool registry**, an **action enum** whose variants
   are the tool vocabulary (dispatched via `match`), and **trait/impl runtimes**
   (a principal `OrderRouter` + a nested `RiskChecker` sub-agent), plus an own
   MCP server and external MCP clients.
3. **Agentic deployment-automation posture** — CI/CD runs Claude Code, OpenAI
   Codex, and a shelled-in aider agent via GitHub Actions, with a mix of safe
   and unsafe config.

Because the agent's tools operate on **account data**, the surfaces interact: a
chat prompt-injection can drive an un-scoped order/position tool into a
cross-account data leak or trade. Those cross-surface chains are the most
interesting thing here to detect.

## Rust agentic idioms exercised (the recall target)

| Construct | Where | Idiom |
|---|---|---|
| Tool registry | `src/agent/tools.rs` | `Lazy<HashMap<&str, ToolSpec>>` static registry |
| Tool vocabulary | `src/agent/action.rs` | `enum AgentAction { ... }` variants dispatched via `match` |
| Principal runtime | `src/agent/order_router.rs` | `trait Agent` + `impl` ReAct loop |
| Sub-agent | `src/agent/risk_checker.rs` | nested runtime, read-only tool subset |
| Own MCP server | `src/mcp_server/` | 2 tools over stdio |
| MCP clients | `.mcp.json` | deskquant-mcp (own) + trent + github (external) |

## False-positive traps (must NOT be inventoried as agents/tools)

- `src/cli/agent_detect.rs` — `CliAgentKind`: a DETECTION enum of coding-agent
  CLIs the tool recognizes by config-file probing. Agent *names*, matched on,
  but no runtime (no LLM, no loop).
- `src/data/bedrock_client.rs` — `BedrockClient`: an LLM transport wrapper, no
  agency.
- `src/agent/handoff.rs` — `run_handoff`/`HandoffChannel`: a demo of the handoff
  edge; its agents are test closures, not runtimes.

## Layer → threat map

| Layer | Carries threats of type |
|---|---|
| `src/api/` | JWT/account-context integrity, authorization |
| `src/data/` | SQL injection, cross-account IDOR, PII handling |
| `infra/` | public buckets, encryption-at-rest, IAM least-privilege, committed creds |
| `src/agent/` + `src/mcp_server/` | prompt injection, RCE/SSRF tools, MCP auth, inter-agent trust |
| `.github/workflows/` | CI/CD coding-agent posture (untrusted-input, token scope, trifecta) |

## Note for maintainers

The ground-truth label set (planted vulns, positive controls, expected inventory
items, cross-surface chains) lives with the Trent design docs, not in this repo.

## Build / test

```bash
cargo build --locked
cargo test --locked
```
