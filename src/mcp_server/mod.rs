//! deskquant-mcp — the desk's own MCP server.
//!
//! Exposes an analyst tool over MCP (stdio): `run_backtest` (runs a strategy
//! script). Ad-hoc SQL (`execute_query`) is intentionally not exposed. Any
//! MCP-compatible host (the order-router in MCP-host mode, or a dev-time coding
//! agent via .mcp.json) can connect and call these.

use crate::data::pg_client::PgClient;

pub const SERVER_NAME: &str = "deskquant-mcp";

/// Tools this server advertises via ListTools.
pub const MCP_TOOLS: &[&str] = &["run_backtest"];

pub struct DeskQuantServer;

impl DeskQuantServer {
    pub fn new(_pg: PgClient) -> Self {
        Self
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
