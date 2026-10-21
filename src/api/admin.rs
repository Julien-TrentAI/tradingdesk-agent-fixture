//! Admin routes (conventional web tier, non-agentic).

use super::chat::verify_jwt;

/// GET /admin/health — liveness. Not agentic.
pub fn health() -> &'static str {
    "ok"
}

/// Failure of an admin request; maps to an HTTP status.
#[derive(Debug, PartialEq, Eq)]
pub enum AdminError {
    /// 401 Unauthorized: missing, malformed, invalid or expired credentials.
    Unauthorized,
    /// 403 Forbidden: authenticated but not an admin.
    Forbidden,
}

impl AdminError {
    pub fn status(&self) -> u16 {
        match self {
            AdminError::Unauthorized => 401,
            AdminError::Forbidden => 403,
        }
    }
}

/// POST /admin/reindex — kicks a maintenance job. Requires an admin JWT in
/// `authorization_header` (`Bearer <jwt>`); every attempt is audit-logged.
pub async fn reindex(authorization_header: &str) -> Result<&'static str, AdminError> {
    let token = authorization_header
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|t| !t.is_empty());
    let claims = match token.and_then(verify_jwt) {
        Some(c) => c,
        None => {
            audit("reindex", "unknown", "failure", "unauthorized");
            return Err(AdminError::Unauthorized);
        }
    };
    if !claims.is_admin {
        audit("reindex", &claims.sub, "failure", "forbidden");
        return Err(AdminError::Forbidden);
    }
    audit("reindex", &claims.sub, "success", "authorized");
    Ok("reindex started")
}

/// Structured audit record for administrative access (emitted via `tracing`).
fn audit(operation: &str, caller: &str, result: &str, reason: &str) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    tracing::info!(
        target: "audit",
        caller,
        timestamp,
        operation,
        result,
        reason,
        "admin access"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_missing_or_bad_token() {
        assert_eq!(reindex("").await, Err(AdminError::Unauthorized));
        assert_eq!(reindex("Bearer garbage").await, Err(AdminError::Unauthorized));
    }
}
