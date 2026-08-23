use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, patch, post},
};

use crate::features::db::{UserData, UserDataPatchRequest, user_data as user_data_db};
use crate::state::AppState;

pub fn user_data_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(get_user_data))
        .route("/", post(post_user_data))
        .route("/", patch(patch_user_data))
        .with_state(state)
}

async fn get_user_data(State(state): State<Arc<AppState>>) -> Result<Json<UserData>, StatusCode> {
    user_data_db::get(state.get_pool())
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn post_user_data(
    State(state): State<Arc<AppState>>,
    Json(user_data): Json<UserData>,
) -> Result<Json<UserData>, StatusCode> {
    user_data_db::create(state.get_pool(), &user_data)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn patch_user_data(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UserDataPatchRequest>,
) -> Result<Json<UserData>, StatusCode> {
    user_data_db::patch(state.get_pool(), &req)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
