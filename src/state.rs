use sqlx::{Pool, Postgres};

use crate::features::db::setup::create_pool;

#[derive(Clone)]
pub struct AppState {
    pool: Pool<Postgres>,
}

impl AppState {
    pub async fn new(connection_string: &str) -> Result<Self, sqlx::Error> {
        let pool = create_pool(connection_string).await?;
        Ok(Self { pool })
    }

    pub fn get_pool(&self) -> &Pool<Postgres> {
        &self.pool
    }
}
