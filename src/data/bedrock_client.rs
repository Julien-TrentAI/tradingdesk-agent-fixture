//! Bedrock LLM client wrapper.
//!
//! FALSE-POSITIVE TRAP (intentional). `BedrockClient` is a thin invoke wrapper
//! around the model API. It has NO tool loop and NO agency — it is the data-tier
//! LLM transport the real runtimes (OrderRouter, RiskChecker) call. A scan must
//! classify it as infrastructure/LLM-client, NOT as an agent runtime, even
//! though it lives next to the agents and holds the model id.

use crate::agent::action::AgentAction;

/// Returns true only for `https://bedrock[-runtime].<region>.amazonaws.com[/...]`.
/// Anything else (http, other hosts, userinfo, ports, look-alike suffixes) is rejected.
pub fn is_aws_bedrock_endpoint(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else { return false };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let Some(mid) = host.strip_suffix(".amazonaws.com") else { return false };
    let Some((service, region)) = mid.split_once('.') else { return false };
    let label_ok = |l: &str| {
        !l.is_empty() && l.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    matches!(service, "bedrock" | "bedrock-runtime") && label_ok(region)
}

/// Builds the HTTP client used for Bedrock. Certificate and hostname
/// verification stay enabled (never `danger_accept_invalid_*`) and only HTTPS
/// is allowed.
pub fn build_http_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder().https_only(true).build()
}

/// Logs a connection attempt (timestamp, endpoint, result). Certificate
/// validation failures are logged at ALERT level for forensic follow-up.
pub fn log_connection_attempt(endpoint: &str, result: &str, cert_failure: bool) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let level = if cert_failure { "ALERT" } else { "INFO" };
    eprintln!("{level} bedrock_connection ts={ts} endpoint={endpoint} result={result}");
}

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
    pub async fn next_action(&self, _tool_menu: &[&str], _user_msg: &str, _scratch: &str) -> AgentAction {
        let endpoint = format!(
            "https://bedrock-runtime.{}.amazonaws.com",
            std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string())
        );
        if !is_aws_bedrock_endpoint(&endpoint) {
            log_connection_attempt(&endpoint, "rejected: not an AWS Bedrock endpoint", false);
            return AgentAction::Finish { answer: "bedrock endpoint rejected".to_string() };
        }
        log_connection_attempt(&endpoint, "stubbed: no request sent", false);
        AgentAction::Finish { answer: "stubbed".to_string() }
    }
}

#[cfg(test)]
mod tests {
    use super::is_aws_bedrock_endpoint as ok;

    #[test]
    fn endpoint_allowlist() {
        assert!(ok("https://bedrock-runtime.us-east-1.amazonaws.com/model/x/invoke"));
        assert!(ok("https://bedrock.eu-west-1.amazonaws.com"));
        assert!(!ok("http://bedrock.us-east-1.amazonaws.com"));
        assert!(!ok("https://bedrock.us-east-1.amazonaws.com.evil.com"));
        assert!(!ok("https://evil.com/bedrock.us-east-1.amazonaws.com"));
        assert!(!ok("https://user@bedrock.us-east-1.amazonaws.com"));
        assert!(!ok("https://example.com"));
    }
}
