pub mod repo;

use anyhow::Context;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, Executor, PgPool};
use std::str::FromStr;

pub use repo::*;

/// One runaway query must not hold a pooled connection forever.
const STATEMENT_TIMEOUT: &str = "30s";

pub async fn connect(database_url: &str) -> anyhow::Result<PgPool> {
    let options = PgConnectOptions::from_str(database_url)
        .context("parse DATABASE_URL")?
        .options([("statement_timeout", STATEMENT_TIMEOUT)]);
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect_with(options)
        .await
        .context("connect postgres")?;
    Ok(pool)
}

pub async fn migrate(pool: &PgPool) -> anyhow::Result<()> {
    // Migrations can outlast the query limit, so they run on their own unlimited connection.
    let mut conn = pool.acquire().await.context("acquire connection")?.detach();
    conn.execute("SET statement_timeout = 0")
        .await
        .context("clear statement timeout")?;
    sqlx::migrate!("../../migrations")
        .run(&mut conn)
        .await
        .context("run migrations")?;
    conn.close().await.context("close migration connection")?;
    Ok(())
}
