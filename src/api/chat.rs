//! POST /v1/chat — the customer entry point.
//!
//! Extracts the caller's account from the JWT, builds an OrderRouter, and runs
//! it on the message. This is where untrusted user text enters the agent loop —
//! the head of the cross-surface chain (prompt-injection -> desk tools ->
//! cross-account data/trade).

use crate::agent::order_router::{Agent, OrderRouter};
use crate::agent::tools::ToolContext;
use crate::data::bedrock_client::BedrockClient;
use crate::data::pg_client::PgClient;

pub struct ChatRequest {
    pub message: String,
    pub jwt: String,
}

pub async fn handle_chat(req: ChatRequest) -> String {
    // NOTE: account is derived from the (unverified in this fixture) JWT; the
    // agent's tools then re-take account ids from the MODEL, breaking scoping.
    let account = account_from_jwt(&req.jwt);
    let ctx = ToolContext {
        pg: PgClient::new("postgres://desk:desk@db/orders"),
        tenant_account: account,
    };
    let router = OrderRouter::new(BedrockClient::new("anthropic.claude-3-5-sonnet-20241022-v2:0"));
    router.run(&ctx, &req.message).await
}

fn account_from_jwt(jwt: &str) -> String {
    // Deliberately naive: trusts an unverified claim substring.
    jwt.split('.').nth(1).unwrap_or("acct-unknown").to_string()
}
