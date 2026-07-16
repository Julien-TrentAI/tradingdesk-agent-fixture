//! The order-desk agent's action (tool) vocabulary.
//!
//! `AgentAction` is the closed set of operations the LLM can request. Each
//! variant carries its payload and is dispatched in the agent loop via `match`
//! to actually execute the operation. This enum IS the agent's tool surface —
//! adding a variant adds a capability the model can invoke.

use serde::{Deserialize, Serialize};

use crate::agent::tools::{self, ToolContext, ToolError};

/// The tool vocabulary. Variants are the tools the model may call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AgentAction {
    /// Place a market/limit order on the desk.
    PlaceOrder { account_id: String, symbol: String, qty: i64, limit: Option<f64> },
    /// Cancel a working order by id.
    CancelOrder { account_id: String, order_id: String },
    /// Get a live quote for a symbol.
    Quote { symbol: String },
    /// Look up an account's current positions.
    LookupPosition { account_id: String },
    /// Read the account's realized/unrealized PnL.
    ReadPnl { account_id: String },
    /// Fetch an external research URL (news, filings) for context.
    FetchResearch { url: String },
    /// Run a shell command in the analyst sandbox (ops/debug).
    RunAnalystShell { command: String },
    /// Hand the task to the risk-check sub-agent for approval.
    DelegateRiskCheck { proposal: String },
    /// Emit the final answer to the user and stop.
    Finish { answer: String },
}

/// The result of executing one action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub ok: bool,
    pub output: String,
}

impl AgentAction {
    /// Dispatch this action to its tool implementation. This `match` is the
    /// consumption site that binds every tool variant to the runtime.
    pub async fn dispatch(&self, ctx: &ToolContext) -> Result<ActionResult, ToolError> {
        let output = match self {
            AgentAction::PlaceOrder { account_id, symbol, qty, limit } => {
                tools::place_order(ctx, account_id, symbol, *qty, *limit).await?
            }
            AgentAction::CancelOrder { account_id, order_id } => {
                tools::cancel_order(ctx, account_id, order_id).await?
            }
            AgentAction::Quote { symbol } => tools::quote(ctx, symbol).await?,
            AgentAction::LookupPosition { account_id } => {
                tools::lookup_position(ctx, account_id).await?
            }
            AgentAction::ReadPnl { account_id } => tools::read_pnl(ctx, account_id).await?,
            AgentAction::FetchResearch { url } => tools::fetch_research(ctx, url).await?,
            AgentAction::RunAnalystShell { command } => {
                tools::run_analyst_shell(ctx, command).await?
            }
            AgentAction::DelegateRiskCheck { proposal } => {
                // Routing action: handled by the runtime, not a leaf tool.
                return Ok(ActionResult { ok: true, output: format!("DELEGATE:{proposal}") });
            }
            AgentAction::Finish { answer } => {
                return Ok(ActionResult { ok: true, output: answer.clone() });
            }
        };
        Ok(ActionResult { ok: true, output })
    }

    pub fn is_finish(&self) -> bool {
        matches!(self, AgentAction::Finish { .. })
    }
}
