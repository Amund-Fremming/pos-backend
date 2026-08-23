use sqlx::{Pool, Postgres};

use crate::features::clients::expo_client::ExpoClient;
use crate::features::db::setup::create_pool;

#[derive(Clone)]
pub struct AppState {
    pool: Pool<Postgres>,
    expo_push_client: ExpoClient,
}

impl AppState {
    pub async fn new(
        connection_string: &str,
        http_client: reqwest::Client,
    ) -> Result<Self, sqlx::Error> {
        let pool = create_pool(connection_string).await?;
        let expo_push_client = ExpoClient::new(http_client);
        Ok(Self {
            pool,
            expo_push_client,
        })
    }

    pub fn get_pool(&self) -> &Pool<Postgres> {
        &self.pool
    }

    pub fn get_expo_push_client(&self) -> &ExpoClient {
        &self.expo_push_client
    }
}
