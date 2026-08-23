use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::features::db::{UserData, UserDataPatchRequest};

pub async fn get(pool: &Pool<Postgres>) -> Result<UserData, sqlx::Error> {
    sqlx::query_as!(
        UserData,
        r#"SELECT id, home_time, home_lat, home_lon, home_display AS "home_display!",
                  work_time, work_lat, work_lon, work_display AS "work_display!", alert_days, push_token
           FROM user_data LIMIT 1"#
    )
    .fetch_one(pool)
    .await
}

pub async fn create(pool: &Pool<Postgres>, user_data: &UserData) -> Result<UserData, sqlx::Error> {
    sqlx::query_as!(
        UserData,
        r#"INSERT INTO user_data
               (home_time, home_lat, home_lon, home_display, work_time, work_lat, work_lon, work_display, alert_days, push_token)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
           RETURNING id, home_time, home_lat, home_lon, home_display AS "home_display!",
                     work_time, work_lat, work_lon, work_display AS "work_display!", alert_days, push_token"#,
        user_data.home_time,
        user_data.home_lat,
        user_data.home_lon,
        user_data.home_display,
        user_data.work_time,
        user_data.work_lat,
        user_data.work_lon,
        user_data.work_display,
        user_data.alert_days,
        user_data.push_token,
    )
    .fetch_one(pool)
    .await
}

pub async fn delete(pool: &Pool<Postgres>, id: Uuid) -> Result<UserData, sqlx::Error> {
    sqlx::query_as!(
        UserData,
        r#"DELETE FROM user_data WHERE id = $1
           RETURNING id, home_time, home_lat, home_lon, home_display AS "home_display!",
                     work_time, work_lat, work_lon, work_display AS "work_display!", alert_days, push_token"#,
        id,
    )
    .fetch_one(pool)
    .await
}

pub async fn patch(
    pool: &Pool<Postgres>,
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
               alert_days = COALESCE($9, alert_days),
               push_token = COALESCE($10, push_token)
           RETURNING id, home_time, home_lat, home_lon, home_display AS "home_display!",
                     work_time, work_lat, work_lon, work_display AS "work_display!", alert_days, push_token"#,
        req.home_time,
        req.home_lat,
        req.home_lon,
        req.home_display,
        req.work_time,
        req.work_lat,
        req.work_lon,
        req.work_display,
        req.alert_days,
        req.push_token,
    )
    .fetch_one(pool)
    .await
}
