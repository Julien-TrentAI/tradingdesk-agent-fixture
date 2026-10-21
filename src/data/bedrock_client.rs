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
    pub async fn next_action(&self, _tool_menu: &[&str], _user_msg: &str, _scratch: &str) -> AgentAction {
        AgentAction::Finish { answer: "stubbed".to_string() }
    }
}
