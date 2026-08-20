mod features;
mod state;

use std::sync::Arc;
use std::time::Duration;

use axum::{Json, Router, routing::get};

use features::routers::weather::weather_router;
use serde_json::json;
use state::AppState;

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build();

    let state = Arc::new(
        AppState::new(&database_url)
            .await
            .expect("Failed to initialize app state"),
    );

    let weather_routes = Router::new().nest("/weather", weather_router(state.clone()));

    let app = Router::new()
        .nest("/api/v1", weather_routes)
        .route("/health", get(health));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:6767")
        .await
        .expect("Failed to bind listener");

    axum::serve(listener, app).await.expect("Server failed");
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "healthy" }))
}
