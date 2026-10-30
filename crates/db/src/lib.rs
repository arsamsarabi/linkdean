pub mod repo;

use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub use repo::*;

pub async fn connect(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await
        .context("connect postgres")?;
    Ok(pool)
}

pub async fn migrate(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .context("run migrations")?;
    Ok(())
}
