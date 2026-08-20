use std::time::Duration;

use sqlx::{Pool, Postgres, postgres::PgPoolOptions};

pub const MAX_POOL_CONNECTIONS: u32 = 20;

pub async fn create_pool(database_url: &str) -> Result<Pool<Postgres>, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(MAX_POOL_CONNECTIONS)
        .connect(database_url)
        .await
}
