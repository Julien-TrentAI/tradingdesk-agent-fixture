//! The Risk Checker — a nested sub-agent runtime.
//!
//! Its own LLM client, its own loop, and a strict subset of the desk tools
//! (read-only: quote, lookup_position, read_pnl). It is invoked by the
//! OrderRouter on a `DelegateRiskCheck` action and returns an approve/deny
//! verdict. Distinct runtime, nested under the router.

use crate::agent::action::AgentAction;
use crate::agent::order_router::{redact_system_info, Budget};
use crate::agent::tools::{ToolContext, ToolError};
use crate::data::bedrock_client::BedrockClient;
use zeroize::Zeroize;

/// Read-only tool subset this sub-agent is allowed to call.
pub const RISK_TOOLS: &[&str] = &["quote", "lookup_position", "read_pnl"];

/// Maximum agent nesting depth (OrderRouter = 0, RiskChecker = 1).
pub const MAX_DEPTH: usize = 2;

pub struct RiskChecker {
    llm: BedrockClient,
    max_steps: usize,
}

impl RiskChecker {
    pub fn new(llm: BedrockClient) -> Self {
        Self { llm, max_steps: 4 }
    }

    /// Runs under the parent's shared `budget`; `depth` is this agent's nesting depth.
    pub async fn run(&self, ctx: &ToolContext, proposal: &str, budget: &mut Budget, depth: usize) -> String {
        if depth > MAX_DEPTH {
            return "error: max agent depth exceeded".to_string();
        }
        let mut scratch = String::new();
        for _ in 0..self.max_steps {
            if budget.timed_out() {
                scratch.zeroize();
                return "error: wall-clock timeout exceeded".to_string();
            }
            let (action, tokens) = self.llm.next_action(RISK_TOOLS, proposal, &scratch).await;
            if !budget.charge(tokens) {
                scratch.zeroize();
                return "token budget exhausted".to_string();
            }
            if let AgentAction::Finish { answer } = &action {
                scratch.zeroize();
                return redact_system_info(answer);
            }
            let allowed: Result<(), ToolError> = match &action {
                AgentAction::Quote { .. }
                | AgentAction::LookupPosition { .. }
                | AgentAction::ReadPnl { .. } => Ok(()),
                _ => Err(ToolError::unauthorized("RiskChecker cannot execute this tool")),
            };
            if let Err(e) = allowed {
                scratch.push_str(&format!("\nerr: {e}"));
                continue;
            }
            match action.dispatch(ctx).await {
                Ok(r) => scratch.push_str(&format!("\n{}", redact_system_info(&r.output))),
                Err(e) => scratch.push_str(&format!("\nerr: {e}")),
            }
        }
        scratch.zeroize();
        "abstain".to_string()
    }
}
