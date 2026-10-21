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
    let message = sanitize_user_input(&req.message);
    router.run(&ctx, &message).await
}

fn account_from_jwt(jwt: &str) -> String {
    // Deliberately naive: trusts an unverified claim substring.
    jwt.split('.').nth(1).unwrap_or("acct-unknown").to_string()
}

/// Phrases commonly used to extract the system prompt or tool registry.
const INJECTION_PATTERNS: &[&str] = &[
    "ignore previous instructions",
    "ignore all previous instructions",
    "ignore instructions",
    "ignore the instructions",
    "system prompt",
    "repeat back",
    "debug mode",
    "show constraints",
];

/// Neutralize common jailbreak / prompt-extraction phrases in user text by
/// replacing each match (case-insensitively) with `[filtered]`.
pub fn sanitize_user_input(msg: &str) -> String {
    let mut out = msg.to_string();
    for pat in INJECTION_PATTERNS {
        loop {
            // ASCII lowercasing preserves byte offsets, so indices map back to `out`.
            let lower = out.to_ascii_lowercase();
            match lower.find(pat) {
                Some(i) => out.replace_range(i..i + pat.len(), "[filtered]"),
                None => break,
            }
        }
    }
    out
}

/// Verified JWT claims relevant to authorization.
pub struct Claims {
    pub sub: String,
    pub is_admin: bool,
}

/// Verify an HS256 JWT (signature with `JWT_SECRET`, `exp` not past) and return its claims.
pub fn verify_jwt(jwt: &str) -> Option<Claims> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    let secret = std::env::var("JWT_SECRET").ok().filter(|s| !s.is_empty())?;
    let mut parts = jwt.split('.');
    let (header, payload, sig) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let hdr = header;
    let header: serde_json::Value = serde_json::from_slice(&B64.decode(hdr).ok()?).ok()?;
    if header.get("alg")?.as_str()? != "HS256" {
        return None;
    }
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(format!("{hdr}.{payload}").as_bytes());
    mac.verify_slice(&B64.decode(sig).ok()?).ok()?;

    let claims: serde_json::Value = serde_json::from_slice(&B64.decode(payload).ok()?).ok()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    if claims.get("exp")?.as_u64()? <= now {
        return None;
    }
    let is_admin = claims.get("admin").and_then(|v| v.as_bool()) == Some(true)
        || claims.get("role").and_then(|v| v.as_str()) == Some("admin");
    Some(Claims { sub: claims.get("sub")?.as_str()?.to_string(), is_admin })
}
