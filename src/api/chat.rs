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
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

pub struct ChatRequest {
    pub message: String,
    pub jwt: String,
}

/// Environment variable holding the PEM-encoded RSA public key used to verify caller JWTs.
pub const JWT_PUBLIC_KEY_ENV: &str = "DESKROUTER_JWT_PUBLIC_KEY";

#[derive(Debug, PartialEq, Eq)]
pub enum ChatError {
    /// JWT missing, malformed, badly signed, expired, or lacking required claims.
    Unauthorized,
    /// Server-side misconfiguration (no verification key available).
    Misconfigured,
}

impl ChatError {
    pub fn status(&self) -> u16 {
        match self {
            ChatError::Unauthorized => 401,
            ChatError::Misconfigured => 500,
        }
    }
}

#[derive(Deserialize)]
struct Claims {
    account_id: String,
    // Required so tokens without them are rejected; `exp` is also validated by the library.
    #[allow(dead_code)]
    iat: u64,
    #[allow(dead_code)]
    exp: u64,
}

pub async fn handle_chat(req: ChatRequest) -> Result<String, ChatError> {
    let public_key = std::env::var(JWT_PUBLIC_KEY_ENV).map_err(|_| ChatError::Misconfigured)?;
    // The account comes only from a cryptographically verified token.
    let account = account_from_jwt(&req.jwt, &public_key)?;
    let ctx = ToolContext {
        pg: PgClient::new("postgres://desk:desk@db/orders"),
        tenant_account: account,
    };
    let router = OrderRouter::new(BedrockClient::new("anthropic.claude-3-5-sonnet-20241022-v2:0"));
    Ok(router.run(&ctx, &req.message).await)
}

/// Verify an RS256 JWT against `public_key_pem` and return its `account_id` claim.
/// Requires `exp`, `iat` and `account_id`; rejects expired tokens and bad signatures.
pub fn account_from_jwt(jwt: &str, public_key_pem: &str) -> Result<String, ChatError> {
    let key = DecodingKey::from_rsa_pem(public_key_pem.as_bytes()).map_err(|_| ChatError::Misconfigured)?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_required_spec_claims(&["exp", "iat"]);
    let data = decode::<Claims>(jwt, &key, &validation).map_err(|_| ChatError::Unauthorized)?;
    if data.claims.account_id.is_empty() {
        return Err(ChatError::Unauthorized);
    }
    Ok(data.claims.account_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;

    const PUB: &str = include_str!("../../tests/fixtures/test_public_key.pem");
    const SIGN: &str = include_str!("../../tests/fixtures/test_signing_key.pem");
    const OTHER: &str = include_str!("../../tests/fixtures/other_signing_key.pem");

    fn now() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
    }

    fn token(key: &str, claims: serde_json::Value) -> String {
        encode(&Header::new(Algorithm::RS256), &claims, &EncodingKey::from_rsa_pem(key.as_bytes()).unwrap()).unwrap()
    }

    fn good() -> serde_json::Value {
        json!({"account_id": "acct-42", "iat": now(), "exp": now() + 600})
    }

    #[test]
    fn valid_token_accepted() {
        assert_eq!(account_from_jwt(&token(SIGN, good()), PUB), Ok("acct-42".to_string()));
    }

    #[test]
    fn wrong_key_rejected() {
        assert_eq!(account_from_jwt(&token(OTHER, good()), PUB), Err(ChatError::Unauthorized));
    }

    #[test]
    fn tampered_payload_rejected() {
        let t = token(SIGN, good());
        let parts: Vec<&str> = t.split('.').collect();
        let forged = format!("{}.{}.{}", parts[0], "eyJhY2NvdW50X2lkIjoiYWNjdC05OSJ9", parts[2]);
        assert_eq!(account_from_jwt(&forged, PUB), Err(ChatError::Unauthorized));
    }

    #[test]
    fn expired_rejected() {
        let c = json!({"account_id": "acct-42", "iat": now() - 7200, "exp": now() - 3600});
        assert_eq!(account_from_jwt(&token(SIGN, c), PUB), Err(ChatError::Unauthorized));
    }

    #[test]
    fn missing_claims_rejected() {
        for c in [
            json!({"iat": now(), "exp": now() + 600}),
            json!({"account_id": "a", "exp": now() + 600}),
            json!({"account_id": "a", "iat": now()}),
        ] {
            assert_eq!(account_from_jwt(&token(SIGN, c), PUB), Err(ChatError::Unauthorized));
        }
    }

    #[test]
    fn legacy_unsigned_format_rejected() {
        assert_eq!(account_from_jwt("hdr.acct-42.sig", PUB), Err(ChatError::Unauthorized));
    }
}
