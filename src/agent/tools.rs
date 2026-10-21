//! Tool implementations + the static tool registry.
//!
//! `TOOL_REGISTRY` is a `Lazy<HashMap>` mapping tool name -> spec. It is what
//! the runtime advertises to the model (names + descriptions) and what the
//! action dispatcher executes. Registering a `ToolSpec` here exposes it to the
//! order-desk agent.

use std::collections::HashMap;
use std::net::IpAddr;

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

pub async fn place_order(
    ctx: &ToolContext,
    account_id: &str,
    symbol: &str,
    qty: i64,
    limit: Option<f64>,
) -> Result<String, ToolError> {
    // VULN: account_id comes from the model, not the verified session — a
    // prompt-injected account_id lets one tenant trade in another's account
    // (cross-account IDOR write).
    ctx.pg
        .insert_order(account_id, symbol, qty, limit)
        .await
        .map_err(|e| ToolError(e.to_string()))?;
    Ok(format!("placed {qty} {symbol} for {account_id}"))
}

pub async fn cancel_order(ctx: &ToolContext, account_id: &str, order_id: &str) -> Result<String, ToolError> {
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
    // VULN: unscoped read — the tool trusts the model-supplied account_id.
    let rows = ctx.pg.positions(account_id).await.map_err(|e| ToolError(e.to_string()))?;
    Ok(rows.join(", "))
}

pub async fn read_pnl(ctx: &ToolContext, account_id: &str) -> Result<String, ToolError> {
    let pnl = ctx.pg.pnl(account_id).await.map_err(|e| ToolError(e.to_string()))?;
    Ok(format!("pnl={pnl}"))
}

/// Hostnames `fetch_research` may contact (exact match, https only).
const RESEARCH_ALLOWED_HOSTS: &[&str] = &["news.example.com", "research.example.com"];

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local() // 169.254.0.0/16, incl. EC2 metadata
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || o[0] == 0
                || (o[0] == 100 && (o[1] & 0xc0) == 64) // 100.64.0.0/10 CGNAT
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_blocked_ip(IpAddr::V4(v4));
            }
            let seg0 = v6.segments()[0];
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (seg0 & 0xfe00) == 0xfc00 // fc00::/7 unique local
                || (seg0 & 0xffc0) == 0xfe80 // fe80::/10 link local
        }
    }
}

/// Validate a model-supplied URL: https only, no credentials, no IP literals in
/// blocked ranges, and the host must be on the allow-list.
fn validate_research_url(raw: &str) -> Result<reqwest::Url, ToolError> {
    let deny = |why: &str| ToolError(format!("url not allowed: {why}"));
    let url = reqwest::Url::parse(raw).map_err(|_| deny("unparseable"))?;
    if url.scheme() != "https" {
        return Err(deny("only https is permitted"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(deny("credentials in url"));
    }
    let host = url.host_str().ok_or_else(|| deny("missing host"))?;
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = bare.parse::<IpAddr>() {
        return Err(deny(if is_blocked_ip(ip) { "blocked address" } else { "ip literals are not permitted" }));
    }
    let d = host.trim_end_matches('.').to_ascii_lowercase();
    if RESEARCH_ALLOWED_HOSTS.contains(&d.as_str()) {
        Ok(url)
    } else {
        Err(deny("host is not on the allow-list"))
    }
}

pub async fn fetch_research(_ctx: &ToolContext, url: &str) -> Result<String, ToolError> {
    let url = validate_research_url(url)?;
    // Redirects are disabled so an allow-listed host cannot bounce us elsewhere.
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| ToolError(e.to_string()))?;
    let body = client.get(url).send().await.map_err(|e| ToolError(e.to_string()))?;
    let text = body.text().await.map_err(|e| ToolError(e.to_string()))?;
    Ok(text.chars().take(500).collect())
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

#[cfg(test)]
mod url_tests {
    use super::validate_research_url;

    #[test]
    fn allows_listed_hosts() {
        assert!(validate_research_url("https://news.example.com/a").is_ok());
    }

    #[test]
    fn blocks_internal_and_unlisted() {
        for u in [
            "http://169.254.169.254/latest/meta-data/",
            "https://169.254.169.254/",
            "https://127.0.0.1/",
            "https://10.0.0.1/",
            "https://[::1]/",
            "https://[::ffff:169.254.169.254]/",
            "https://evil.example.org/",
            "https://news.example.com@evil.example.org/",
            "http://news.example.com/",
            "file:///etc/passwd",
        ] {
            assert!(validate_research_url(u).is_err(), "{u}");
        }
    }
}
