use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use uuid::Uuid;

use crate::clients::weather_client::{JacketInterval, Weather};
use crate::db::user_data as user_data_db;
use crate::schedule;
use crate::state::AppState;

pub fn weather_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/{id}", get(get_weather))
        .route("/{id}/jacket-intervals", get(get_jacket_intervals))
        .with_state(state)
}

/// Default forecast for the home screen — weather around the user's home departure.
async fn get_weather(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Weather>, StatusCode> {
    let user = user_data_db::get_by_id(state.get_pool(), id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    tracing::debug!(
        home_time = ?user.home_time,
        home_lat = user.home_lat,
        home_lon = user.home_lon,
        work_time = ?user.work_time,
        work_lat = user.work_lat,
        work_lon = user.work_lon,
        commute_minutes = user.commute_minutes,
        "travel values"
    );

    let weather = state
        .get_weather_client()
        .get_weather(
            schedule::now_oslo().date_naive(),
            user.home_time,
            user.home_lat,
            user.home_lon,
            user.work_time,
            user.work_lat,
            user.work_lon,
            user.commute_minutes,
        )
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    Ok(Json(weather))
}

/// Off-day view: today's wet stretches around home, for days the user isn't commuting.
async fn get_jacket_intervals(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<JacketInterval>>, StatusCode> {
    let user = user_data_db::get_by_id(state.get_pool(), id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let intervals = state
        .get_weather_client()
        .jacket_intervals(
            schedule::now_oslo().date_naive(),
            user.home_lat,
            user.home_lon,
        )
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    Ok(Json(intervals))
}
