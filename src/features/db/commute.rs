use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::features::db::{Commute, CommutePatchRequest};

const SELECT_COLUMNS: &str =
    "id, home_time, home_lat, home_lon, work_time, work_lat, work_lon, alert_days";

pub async fn get(pool: &Pool<Postgres>) -> Result<Commute, sqlx::Error> {
    sqlx::query_as::<_, Commute>(&format!("SELECT {SELECT_COLUMNS} FROM commute LIMIT 1"))
        .fetch_one(pool)
        .await
}

pub async fn create(pool: &Pool<Postgres>, commute: &Commute) -> Result<Commute, sqlx::Error> {
    sqlx::query_as::<_, Commute>(&format!(
        "INSERT INTO commute (home_time, home_lat, home_lon, work_time, work_lat, work_lon, alert_days)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(commute.home_time)
    .bind(commute.home_lat)
    .bind(commute.home_lon)
    .bind(commute.work_time)
    .bind(commute.work_lat)
    .bind(commute.work_lon)
    .bind(&commute.alert_days)
    .fetch_one(pool)
    .await
}

pub async fn delete(pool: &Pool<Postgres>, id: Uuid) -> Result<Commute, sqlx::Error> {
    sqlx::query_as::<_, Commute>(&format!(
        "DELETE FROM commute WHERE id = $1 RETURNING {SELECT_COLUMNS}"
    ))
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn patch(
    pool: &Pool<Postgres>,
    req: &CommutePatchRequest,
) -> Result<Commute, sqlx::Error> {
    sqlx::query_as::<_, Commute>(&format!(
        "UPDATE commute
         SET
             home_time = COALESCE($1, home_time),
             home_lat = COALESCE($2, home_lat),
             home_lon = COALESCE($3, home_lon),
             work_time = COALESCE($4, work_time),
             work_lat = COALESCE($5, work_lat),
             work_lon = COALESCE($6, work_lon),
             alert_days = COALESCE($7, alert_days)
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(req.home_time)
    .bind(req.home_lat)
    .bind(req.home_lon)
    .bind(req.work_time)
    .bind(req.work_lat)
    .bind(req.work_lon)
    .bind(&req.alert_days)
    .fetch_one(pool)
    .await
}
