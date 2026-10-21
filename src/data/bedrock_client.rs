//! Bedrock LLM client wrapper.
//!
//! FALSE-POSITIVE TRAP (intentional). `BedrockClient` is a thin invoke wrapper
//! around the model API. It has NO tool loop and NO agency — it is the data-tier
//! LLM transport the real runtimes (OrderRouter, RiskChecker) call. A scan must
//! classify it as infrastructure/LLM-client, NOT as an agent runtime, even
//! though it lives next to the agents and holds the model id.

use crate::agent::action::AgentAction;

pub struct BedrockClient {
    model_id: String,
}

impl BedrockClient {
    pub fn new(model_id: &str) -> Self {
        Self { model_id: model_id.to_string() }
    }

    pub fn model(&self) -> &str {
        &self.model_id
    }

    /// Ask the model for the next action. In this fixture the transport is
    /// stubbed; a real deployment would `bedrock:InvokeModel` here. No loop,
    /// no dispatch — the caller (a runtime) owns the agency.
    ///
    /// Contract: `_user_msg` and `_scratch` must be sanitized by the caller
    /// before invocation. This transport layer performs no filtering, to remain
    /// protocol-agnostic.
    ///
    /// Returns the action plus the tokens consumed by the call, so callers can
    /// enforce a budget. The stub estimates usage at ~4 bytes per token.
    pub async fn next_action(&self, tool_menu: &[&str], user_msg: &str, scratch: &str) -> (AgentAction, usize) {
        let answer = "stubbed".to_string();
        let input: usize = tool_menu.iter().map(|t| t.len()).sum::<usize>() + user_msg.len() + scratch.len();
        let tokens = (input + answer.len()).div_ceil(4);
        (AgentAction::Finish { answer }, tokens)
    }
}
