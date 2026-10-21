//! deskrouter — a trading-desk order-routing service with an LLM agent.
//!
//! SECURITY VALIDATION FIXTURE — DELIBERATELY VULNERABLE. Do not deploy.
//!
//! Wires the conventional web/data tier to the agent runtimes. The principal
//! runtime is the OrderRouter (src/agent/order_router.rs); it delegates to the
//! RiskChecker sub-agent and may host the deskquant MCP server's tools.

use deskrouter::api::chat::{handle_chat, ChatRequest};

#[tokio::main]
async fn main() {
    // A single illustrative request; a real service would bind axum routes.
    let resp = handle_chat(ChatRequest {
        message: "what is my AAPL position?".to_string(),
        jwt: "hdr.acct-42.sig".to_string(),
    })
    .await;
    match resp {
        Ok(text) => println!("{text}"),
        Err(e) => eprintln!("request rejected: HTTP {}", e.status()),
    }
}
