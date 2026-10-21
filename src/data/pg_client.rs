//! Postgres data client (orders, positions, PnL).
//!
//! All statements are parameterized (`$n` placeholders); no caller-supplied
//! value is ever interpolated into SQL text.

use tokio::sync::OnceCell;
use tokio_postgres::{Client, NoTls};

pub struct PgClient {
    dsn: String,
    client: OnceCell<Client>,
}

#[derive(Debug)]
pub struct DbError(pub String);

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<tokio_postgres::Error> for DbError {
    fn from(e: tokio_postgres::Error) -> Self {
        DbError(e.to_string())
    }
}

impl PgClient {
    pub fn new(dsn: &str) -> Self {
        Self { dsn: dsn.to_string(), client: OnceCell::new() }
    }

    /// Live connection, established on first use and reused afterwards.
    async fn conn(&self) -> Result<&Client, DbError> {
        self.client
            .get_or_try_init(|| async {
                let (client, connection) = tokio_postgres::connect(&self.dsn, NoTls).await?;
                tokio::spawn(async move {
                    let _ = connection.await;
                });
                Ok::<_, tokio_postgres::Error>(client)
            })
            .await
            .map_err(DbError::from)
    }

    pub async fn insert_order(
        &self,
        account_id: &str,
        symbol: &str,
        qty: i64,
        limit: Option<f64>,
    ) -> Result<(), DbError> {
        let client = self.conn().await?;
        client
            .execute(
                "INSERT INTO orders(account_id, symbol, qty, limit_px) VALUES ($1, $2, $3, $4)",
                &[&account_id, &symbol, &qty, &limit.unwrap_or(0.0)],
            )
            .await?;
        Ok(())
    }

    pub async fn cancel_order(&self, account_id: &str, order_id: &str) -> Result<(), DbError> {
        let client = self.conn().await?;
        client
            .execute(
                "UPDATE orders SET status='cancelled' WHERE id::text=$1 AND account_id=$2",
                &[&order_id, &account_id],
            )
            .await?;
        Ok(())
    }

    pub async fn positions(&self, account_id: &str) -> Result<Vec<String>, DbError> {
        let client = self.conn().await?;
        let rows = client
            .query("SELECT symbol, qty FROM positions WHERE account_id=$1", &[&account_id])
            .await?;
        rows.iter()
            .map(|r| {
                let symbol: String = r.try_get(0)?;
                let qty: i64 = r.try_get(1)?;
                Ok(format!("{symbol}:{qty}"))
            })
            .collect::<Result<_, tokio_postgres::Error>>()
            .map_err(DbError::from)
    }

    pub async fn pnl(&self, account_id: &str) -> Result<f64, DbError> {
        let client = self.conn().await?;
        let row = client
            .query_one(
                "SELECT COALESCE(sum(pnl), 0)::float8 FROM fills WHERE account_id=$1",
                &[&account_id],
            )
            .await?;
        Ok(row.try_get(0)?)
    }
}
