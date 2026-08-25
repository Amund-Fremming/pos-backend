use std::sync::Arc;

use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use serde::Deserialize;

use crate::db::user_data as user_data_db;
use crate::state::AppState;

pub fn push_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/notify", post(notify))
        .route("/message", post(message))
        .with_state(state)
}

#[derive(Deserialize)]
struct NotifyRequest {
    title: String,
    body: String,
}

async fn notify(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NotifyRequest>,
) -> Result<StatusCode, StatusCode> {
    let Some(token) = push_token(&state).await? else {
        return Ok(StatusCode::NO_CONTENT);
    };

    state
        .get_expo_push_client()
        .send(&[token], &req.title, &req.body)
        .await
        .map(|_| StatusCode::OK)
        .map_err(|_| StatusCode::BAD_GATEWAY)
}

async fn message(State(state): State<Arc<AppState>>) -> Result<StatusCode, StatusCode> {
    let Some(token) = push_token(&state).await? else {
        return Ok(StatusCode::NO_CONTENT);
    };

    state
        .get_expo_push_client()
        .send(
            &[token],
            "TEST",
            "This is a test notification from pos-backend.",
        )
        .await
        .map(|_| StatusCode::OK)
        .map_err(|_| StatusCode::BAD_GATEWAY)
}

async fn push_token(state: &Arc<AppState>) -> Result<Option<String>, StatusCode> {
    user_data_db::get(state.get_pool())
        .await
        .map(|user_data| user_data.push_token)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
