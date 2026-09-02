mod clients;
mod cron;
mod db;
mod routers;
mod state;

use std::sync::Arc;
use std::time::Duration;

use axum::{Json, Router, routing::get};

use routers::user_data::user_data_router;
use routers::weather::weather_router;
use serde_json::json;
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("pos_backend=trace,info")),
        )
        .init();

    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;

    let state = Arc::new(
        AppState::new(&database_url, http_client)
            .await
            .expect("Failed to initialize app state"),
    );

    tokio::spawn(weather_cron::spawn(state.clone()));

    let weather_routes = Router::new().nest("/weather", weather_router(state.clone()));
    let user_data_routes = Router::new().nest("/user-data", user_data_router(state.clone()));

    let app = Router::new()
        .nest("/api/v1", weather_routes)
        .nest("/api/v1", user_data_routes)
        .route("/health", get(health));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:6969")
        .await
        .expect("Failed to bind listener");

    tracing::info!("listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await.expect("Server failed");

    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "healthy" }))
}
