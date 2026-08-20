use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, patch, post},
};

use crate::features::db::{Commute, CommutePatchRequest, commute as commute_db};
use crate::state::AppState;

pub fn commute_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(get_commute))
        .route("/", post(post_commute))
        .route("/", patch(patch_commute))
        .with_state(state)
}

async fn get_commute(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Commute>, StatusCode> {
    commute_db::get(state.get_pool())
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn post_commute(
    State(state): State<Arc<AppState>>,
    Json(commute): Json<Commute>,
) -> Result<Json<Commute>, StatusCode> {
    commute_db::create(state.get_pool(), &commute)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn patch_commute(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CommutePatchRequest>,
) -> Result<Json<Commute>, StatusCode> {
    commute_db::patch(state.get_pool(), &req)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
