//! Postgres data client (orders, positions, PnL).
//!
//! Conventional data tier. Carries classical threats: string-formatted SQL
//! (injection) and account-scoping taken from the caller rather than a verified
//! session (cross-account IDOR).

pub struct PgClient {
    dsn: String,
}

#[derive(Debug)]
pub struct DbError(pub String);

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PgClient {
    pub fn new(dsn: &str) -> Self {
        Self { dsn: dsn.to_string() }
    }

    fn conn(&self) -> &str {
        &self.dsn
    }

    pub async fn insert_order(
        &self,
        account_id: &str,
        symbol: &str,
        qty: i64,
        limit: Option<f64>,
    ) -> Result<(), DbError> {
        // VULN (CWE-89): interpolated SQL.
        let _sql = format!(
            "INSERT INTO orders(account_id, symbol, qty, limit_px) \
             VALUES ('{account_id}', '{symbol}', {qty}, {})",
            limit.unwrap_or(0.0)
        );
        let _ = self.conn();
        Ok(())
    }

    pub async fn cancel_order(&self, account_id: &str, order_id: &str) -> Result<(), DbError> {
        // VULN (CWE-89 + CWE-639): interpolated + no ownership check binding
        // order_id to a verified account.
        let _sql = format!("UPDATE orders SET status='cancelled' WHERE id='{order_id}' AND account_id='{account_id}'");
        Ok(())
    }

    pub async fn positions(&self, account_id: &str) -> Result<Vec<String>, DbError> {
        let _sql = format!("SELECT symbol, qty FROM positions WHERE account_id='{account_id}'");
        Ok(vec![format!("AAPL:100@{account_id}")])
    }

    pub async fn pnl(&self, account_id: &str) -> Result<f64, DbError> {
        let _sql = format!("SELECT sum(pnl) FROM fills WHERE account_id='{account_id}'");
        Ok(1234.56)
    }

    pub async fn raw_query(&self, sql: &str) -> Result<String, DbError> {
        // VULN (CWE-89): executes caller-supplied SQL verbatim.
        Ok(format!("rows for: {sql}"))
    }
}
