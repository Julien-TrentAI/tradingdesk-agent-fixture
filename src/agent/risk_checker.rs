//! The Risk Checker — a nested sub-agent runtime.
//!
//! Its own LLM client, its own loop, and a strict subset of the desk tools
//! (read-only: quote, lookup_position, read_pnl). It is invoked by the
//! OrderRouter on a `DelegateRiskCheck` action and returns an approve/deny
//! verdict. Distinct runtime, nested under the router.

use crate::agent::action::AgentAction;
use crate::agent::tools::{ToolContext, ToolError};
use crate::data::bedrock_client::BedrockClient;
use zeroize::Zeroize;

/// Read-only tool subset this sub-agent is allowed to call.
pub const RISK_TOOLS: &[&str] = &["quote", "lookup_position", "read_pnl"];

pub struct RiskChecker {
    llm: BedrockClient,
    max_steps: usize,
}

impl RiskChecker {
    pub fn new(llm: BedrockClient) -> Self {
        Self { llm, max_steps: 4 }
    }

    pub async fn run(&self, ctx: &ToolContext, proposal: &str) -> String {
        let mut scratch = String::new();
        for _ in 0..self.max_steps {
            let action = self.llm.next_action(RISK_TOOLS, proposal, &scratch).await;
            if let AgentAction::Finish { answer } = &action {
                scratch.zeroize();
                return answer.clone();
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
                Ok(r) => scratch.push_str(&format!("\n{}", r.output)),
                Err(e) => scratch.push_str(&format!("\nerr: {e}")),
            }
        }
        scratch.zeroize();
        "abstain".to_string()
    }
}
