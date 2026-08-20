use std::sync::Arc;

use axum::{Router, extract::State, http::StatusCode, routing::get};

use crate::state::AppState;

pub fn weather_router(state: Arc<AppState>) -> Router {
    Router::new().route("/", get(get_weather)).with_state(state)
}

async fn get_weather(State(_state): State<Arc<AppState>>) -> StatusCode {
    StatusCode::OK
}
