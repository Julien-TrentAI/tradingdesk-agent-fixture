//! Tool implementations + the static tool registry.
//!
//! `TOOL_REGISTRY` is a `Lazy<HashMap>` mapping tool name -> spec. It is what
//! the runtime advertises to the model (names + descriptions) and what the
//! action dispatcher executes. Registering a `ToolSpec` here exposes it to the
//! order-desk agent.

use std::collections::HashMap;

use once_cell::sync::Lazy;

use crate::data::pg_client::PgClient;

pub struct ToolContext {
    pub pg: PgClient,
    pub tenant_account: String,
}

#[derive(Debug)]
pub struct ToolError(pub String);

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for ToolError {}

impl ToolError {
    pub fn unauthorized(msg: &str) -> Self {
        ToolError(format!("Unauthorized: {msg}"))
    }
}

/// Static description of a tool the model may call.
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    /// read/write/exec/egress — used only for posture description.
    pub capability: &'static str,
}

/// The desk's tool registry. Every entry is a callable tool bound to the
/// order-router runtime (see order_router.rs, which reads this map to build the
/// model's tool list).
pub static TOOL_REGISTRY: Lazy<HashMap<&'static str, ToolSpec>> = Lazy::new(|| {
    let mut m = HashMap::new();
    for spec in [
        ToolSpec { name: "place_order", description: "Place a market/limit order", capability: "write" },
        ToolSpec { name: "cancel_order", description: "Cancel a working order", capability: "write" },
        ToolSpec { name: "quote", description: "Live quote for a symbol", capability: "read" },
        ToolSpec { name: "lookup_position", description: "Account positions", capability: "read" },
        ToolSpec { name: "read_pnl", description: "Account realized/unrealized PnL", capability: "read" },
        ToolSpec { name: "fetch_research", description: "Fetch an external research URL", capability: "egress" },
        ToolSpec { name: "run_analyst_shell", description: "Run a shell command in the analyst sandbox", capability: "exec" },
    ] {
        m.insert(spec.name, spec);
    }
    m
});

/// Reject any model-supplied account that is not the session's verified account.
fn ensure_own_account(ctx: &ToolContext, account_id: &str) -> Result<(), ToolError> {
    if account_id != ctx.tenant_account {
        return Err(ToolError("account_id does not match the authenticated account".into()));
    }
    Ok(())
}

pub async fn place_order(
    ctx: &ToolContext,
    account_id: &str,
    symbol: &str,
    qty: i64,
    limit: Option<f64>,
) -> Result<String, ToolError> {
    ensure_own_account(ctx, account_id)?;
    ctx.pg
        .insert_order(account_id, symbol, qty, limit)
        .await
        .map_err(|e| ToolError(e.to_string()))?;
    Ok(format!("placed {qty} {symbol} for {account_id}"))
}

pub async fn cancel_order(ctx: &ToolContext, account_id: &str, order_id: &str) -> Result<String, ToolError> {
    ensure_own_account(ctx, account_id)?;
    ctx.pg
        .cancel_order(account_id, order_id)
        .await
        .map_err(|e| ToolError(e.to_string()))?;
    Ok(format!("cancelled {order_id}"))
}

pub async fn quote(_ctx: &ToolContext, symbol: &str) -> Result<String, ToolError> {
    Ok(format!("{symbol}: 100.00/100.05"))
}

pub async fn lookup_position(ctx: &ToolContext, account_id: &str) -> Result<String, ToolError> {
    ensure_own_account(ctx, account_id)?;
    let rows = ctx.pg.positions(account_id).await.map_err(|e| ToolError(e.to_string()))?;
    Ok(rows.join(", "))
}

pub async fn read_pnl(ctx: &ToolContext, account_id: &str) -> Result<String, ToolError> {
    ensure_own_account(ctx, account_id)?;
    let pnl = ctx.pg.pnl(account_id).await.map_err(|e| ToolError(e.to_string()))?;
    Ok(format!("pnl={pnl}"))
}

/// Domains `fetch_research` may contact (exact host or any subdomain). Override
/// with a comma-separated `RESEARCH_ALLOWED_DOMAINS` environment variable.
const DEFAULT_RESEARCH_DOMAINS: &[&str] = &["sec.gov", "federalreserve.gov", "bls.gov", "treasury.gov"];

fn research_domains() -> Vec<String> {
    match std::env::var("RESEARCH_ALLOWED_DOMAINS") {
        Ok(v) if !v.trim().is_empty() => v
            .split(',')
            .map(|d| d.trim().trim_start_matches('.').to_ascii_lowercase())
            .filter(|d| !d.is_empty())
            .collect(),
        _ => DEFAULT_RESEARCH_DOMAINS.iter().map(|d| d.to_string()).collect(),
    }
}

/// Validate a model-supplied URL against the allow-list. Only https URLs with a
/// DNS host on the allow-list are accepted; IP literals (which covers private
/// ranges and metadata endpoints such as 169.254.169.254) are always rejected.
fn validate_research_url(url: &str, allowed: &[String]) -> Result<reqwest::Url, ToolError> {
    let parsed = reqwest::Url::parse(url).map_err(|_| ToolError("invalid URL".into()))?;
    if parsed.scheme() != "https" {
        return Err(ToolError("only https URLs are allowed".into()));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(ToolError("URL credentials are not allowed".into()));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| ToolError("URL has no host".into()))?
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host.starts_with('[') || host.parse::<std::net::IpAddr>().is_ok() {
        return Err(ToolError("IP address hosts are not allowed".into()));
    }
    let permitted = allowed
        .iter()
        .any(|d| host == *d || host.strip_suffix(d.as_str()).is_some_and(|p| p.ends_with('.')));
    if !permitted {
        return Err(ToolError(format!("host {host} is not on the research allow-list")));
    }
    Ok(parsed)
}

pub async fn fetch_research(_ctx: &ToolContext, url: &str) -> Result<String, ToolError> {
    let parsed = validate_research_url(url, &research_domains())?;
    // Redirects are disabled so an allow-listed host cannot bounce us elsewhere.
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| ToolError(e.to_string()))?;
    let body = client.get(parsed).send().await.map_err(|e| ToolError(e.to_string()))?;
    let text = body.text().await.map_err(|e| ToolError(e.to_string()))?;
    Ok(text.chars().take(500).collect())
}

#[cfg(test)]
mod research_url_tests {
    use super::*;

    fn allowed() -> Vec<String> {
        vec!["sec.gov".to_string()]
    }

    #[test]
    fn accepts_allowed_hosts() {
        assert!(validate_research_url("https://sec.gov/x", &allowed()).is_ok());
        assert!(validate_research_url("https://www.SEC.gov/x", &allowed()).is_ok());
    }

    #[test]
    fn rejects_unsafe_urls() {
        for u in [
            "http://sec.gov/x",
            "https://169.254.169.254/latest/meta-data/",
            "https://10.0.0.1/",
            "https://127.0.0.1/",
            "https://[::1]/",
            "https://evilsec.gov/",
            "https://sec.gov.evil.com/",
            "https://sec.gov@evil.com/",
            "https://metadata.google.internal/",
            "file:///etc/passwd",
            "not a url",
        ] {
            assert!(validate_research_url(u, &allowed()).is_err(), "{u}");
        }
    }
}

pub async fn run_analyst_shell(_ctx: &ToolContext, command: &str) -> Result<String, ToolError> {
    // VULN: RCE — arbitrary command execution via a model-chosen string.
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|e| ToolError(e.to_string()))?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}
