//! The Order Router — the desk's principal agent runtime.
//!
//! Implements the `Agent` trait: an LLM-driven ReAct loop that reads the tool
//! registry, asks the model for the next `AgentAction`, dispatches it, and
//! feeds the result back until the model emits `Finish`. On a
//! `DelegateRiskCheck` action it invokes the nested `RiskChecker` sub-agent.

use crate::agent::action::{ActionResult, AgentAction};
use crate::agent::risk_checker::RiskChecker;
use crate::agent::tools::{ToolContext, TOOL_REGISTRY};
use crate::data::bedrock_client::BedrockClient;

/// A live, LLM-driving agent runtime.
#[async_trait::async_trait]
pub trait Agent {
    fn name(&self) -> &str;
    async fn run(&self, ctx: &ToolContext, user_msg: &str) -> String;
}

pub struct OrderRouter {
    llm: BedrockClient,
    risk: RiskChecker,
    max_steps: usize,
}

impl OrderRouter {
    pub fn new(llm: BedrockClient) -> Self {
        // The sub-agent shares the model client but has its own loop + tool set.
        let risk = RiskChecker::new(BedrockClient::new("anthropic.claude-3-5-haiku-20241022-v1:0"));
        Self { llm, risk, max_steps: 12 }
    }

    /// The tool names advertised to the model, sourced from the registry.
    fn tool_menu(&self) -> Vec<&'static str> {
        TOOL_REGISTRY.values().map(|s| s.name).collect()
    }
}

#[async_trait::async_trait]
impl Agent for OrderRouter {
    fn name(&self) -> &str {
        "order-router"
    }

    async fn run(&self, ctx: &ToolContext, user_msg: &str) -> String {
        let menu = self.tool_menu();
        let mut scratch = String::new();
        for _ in 0..self.max_steps {
            // Ask the model for the next action given the tool menu + scratch.
            let action: AgentAction = self.llm.next_action(&menu, user_msg, &scratch).await;

            if let AgentAction::DelegateRiskCheck { proposal } = &action {
                // Hand off to the nested risk-check runtime.
                let verdict = self.risk.run(ctx, proposal).await;
                scratch.push_str(&format!("\nrisk-verdict: {verdict}"));
                continue;
            }

            let ActionResult { ok, output } = match action.dispatch(ctx).await {
                Ok(r) => r,
                Err(e) => ActionResult { ok: false, output: e.to_string() },
            };
            if action.is_finish() {
                return output;
            }
            scratch.push_str(&format!("\n[{ok}] {output}"));
        }
        "step budget exhausted".to_string()
    }
}
