//! Admin routes (conventional web tier, non-agentic).

/// GET /admin/health — liveness. Not agentic.
pub fn health() -> &'static str {
    "ok"
}

/// POST /admin/reindex — kicks a maintenance job. VULN: no authz check.
pub async fn reindex() -> &'static str {
    "reindex started"
}
