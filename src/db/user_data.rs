use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::db::{UserData, UserDataPatchRequest};

/// Blind single-row fetch — used internally (e.g. push notify) where there's
/// only ever one row and no client-supplied id.
pub async fn get(pool: &Pool<Postgres>) -> Result<UserData, sqlx::Error> {
    sqlx::query_as!(
        UserData,
        r#"SELECT id, home_time, home_lat, home_lon, home_display AS "home_display!",
                  work_time, work_lat, work_lon, work_display AS "work_display!", commute_minutes, alert_days, push_token
           FROM user_data LIMIT 1"#
    )
    .fetch_one(pool)
    .await
}

/// Rows eligible for a departure alert — anything with a push token registered.
pub async fn get_all_with_push_token(pool: &Pool<Postgres>) -> Result<Vec<UserData>, sqlx::Error> {
    sqlx::query_as!(
        UserData,
        r#"SELECT id, home_time, home_lat, home_lon, home_display AS "home_display!",
                  work_time, work_lat, work_lon, work_display AS "work_display!", commute_minutes, alert_days, push_token
           FROM user_data WHERE push_token IS NOT NULL"#
    )
    .fetch_all(pool)
    .await
}

pub async fn get_by_id(pool: &Pool<Postgres>, id: Uuid) -> Result<UserData, sqlx::Error> {
    sqlx::query_as!(
        UserData,
        r#"SELECT id, home_time, home_lat, home_lon, home_display AS "home_display!",
                  work_time, work_lat, work_lon, work_display AS "work_display!", commute_minutes, alert_days, push_token
           FROM user_data WHERE id = $1"#,
        id,
    )
    .fetch_one(pool)
    .await
}

pub async fn create(pool: &Pool<Postgres>, user_data: &UserData) -> Result<UserData, sqlx::Error> {
    sqlx::query_as!(
        UserData,
        r#"INSERT INTO user_data
               (home_time, home_lat, home_lon, home_display, work_time, work_lat, work_lon, work_display, commute_minutes, alert_days, push_token)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           RETURNING id, home_time, home_lat, home_lon, home_display AS "home_display!",
                     work_time, work_lat, work_lon, work_display AS "work_display!", commute_minutes, alert_days, push_token"#,
        user_data.home_time,
        user_data.home_lat,
        user_data.home_lon,
        user_data.home_display,
        user_data.work_time,
        user_data.work_lat,
        user_data.work_lon,
        user_data.work_display,
        user_data.commute_minutes,
        user_data.alert_days,
        user_data.push_token,
    )
    .fetch_one(pool)
    .await
}

pub async fn patch_by_id(
    pool: &Pool<Postgres>,
    id: Uuid,
    req: &UserDataPatchRequest,
) -> Result<UserData, sqlx::Error> {
    sqlx::query_as!(
        UserData,
        r#"UPDATE user_data
           SET
               home_time = COALESCE($1, home_time),
               home_lat = COALESCE($2, home_lat),
               home_lon = COALESCE($3, home_lon),
               home_display = COALESCE($4, home_display),
               work_time = COALESCE($5, work_time),
               work_lat = COALESCE($6, work_lat),
               work_lon = COALESCE($7, work_lon),
               work_display = COALESCE($8, work_display),
               commute_minutes = COALESCE($9, commute_minutes),
               alert_days = COALESCE($10, alert_days),
               push_token = COALESCE($11, push_token)
           WHERE id = $12
           RETURNING id, home_time, home_lat, home_lon, home_display AS "home_display!",
                     work_time, work_lat, work_lon, work_display AS "work_display!", commute_minutes, alert_days, push_token"#,
        req.home_time,
        req.home_lat,
        req.home_lon,
        req.home_display,
        req.work_time,
        req.work_lat,
        req.work_lon,
        req.work_display,
        req.commute_minutes,
        req.alert_days,
        req.push_token,
        id,
    )
    .fetch_one(pool)
    .await
}
