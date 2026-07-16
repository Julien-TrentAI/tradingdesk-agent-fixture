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
    /// VULN: the script path/body is executed as a shell command — RCE over MCP.
    pub async fn run_backtest(&self, script: &str) -> Result<String, String> {
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(script)
            .output()
            .map_err(|e| e.to_string())?;
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }
}
