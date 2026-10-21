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

pub async fn fetch_research(_ctx: &ToolContext, url: &str) -> Result<String, ToolError> {
    // VULN: SSRF — the model chooses the URL and it is fetched with no
    // allow-list, so it can reach internal metadata endpoints.
    let body = reqwest::get(url).await.map_err(|e| ToolError(e.to_string()))?;
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
