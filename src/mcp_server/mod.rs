//! deskquant-mcp — the desk's own MCP server.
//!
//! Exposes two analyst tools over MCP (stdio): `execute_query` (ad-hoc SQL over
//! the analytics replica) and `run_backtest` (runs a strategy script). Any
//! MCP-compatible host (the order-router in MCP-host mode, or a dev-time coding
//! agent via .mcp.json) can connect and call these.

use crate::data::pg_client::PgClient;

pub const SERVER_NAME: &str = "deskquant-mcp";

/// Tools this server advertises via ListTools.
pub const MCP_TOOLS: &[&str] = &["execute_query", "run_backtest"];

pub struct DeskQuantServer {
    pg: PgClient,
}

impl DeskQuantServer {
    pub fn new(pg: PgClient) -> Self {
        Self { pg }
    }

    /// MCP tool: run an arbitrary read query against the analytics replica.
    /// VULN: SQL is taken verbatim from the caller — SQL injection over MCP.
    pub async fn execute_query(&self, sql: &str) -> Result<String, String> {
        self.pg.raw_query(sql).await.map_err(|e| e.to_string())
    }

    /// MCP tool: run a strategy backtest script.
    ///
    /// The caller must present the token configured in `DESKQUANT_MCP_TOKEN`,
    /// and `script` must be one of `APPROVED_BACKTEST_SCRIPTS`. The script is
    /// run directly with `python3` (no shell).
    pub async fn run_backtest(&self, token: &str, script: &str) -> Result<String, String> {
        if !is_authorized(token) {
            return Err("unauthorized".to_string());
        }
        if !APPROVED_BACKTEST_SCRIPTS.contains(&script) {
            return Err("script not approved".to_string());
        }
        let out = std::process::Command::new("python3")
            .arg(script)
            .output()
            .map_err(|e| e.to_string())?;
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }
}

/// Backtest scripts the MCP server may execute.
pub const APPROVED_BACKTEST_SCRIPTS: &[&str] = &["/opt/backtest/approved_script.py"];

/// Constant-time-ish comparison against the token from the environment.
/// Fails closed when no token is configured.
fn is_authorized(token: &str) -> bool {
    match std::env::var("DESKQUANT_MCP_TOKEN") {
        Ok(expected) if !expected.is_empty() && expected.len() == token.len() => expected
            .bytes()
            .zip(token.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0,
        _ => false,
    }
}
