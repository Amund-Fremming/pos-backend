use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::db::{UserData, UserDataPatchRequest};

/// Rows eligible for a departure alert — anything with a push token registered.
pub async fn get_all_with_push_token(pool: &Pool<Postgres>) -> Result<Vec<UserData>, sqlx::Error> {
    sqlx::query_as!(
        UserData,
        r#"SELECT id, home_time, home_lat, home_lon, home_display AS "home_display!",
                  work_time, work_lat, work_lon, work_display AS "work_display!", commute_minutes, alert_days, push_token, last_alerted_date
           FROM user_data WHERE push_token IS NOT NULL"#
    )
    .fetch_all(pool)
    .await
}

pub async fn get_by_id(pool: &Pool<Postgres>, id: Uuid) -> Result<UserData, sqlx::Error> {
    sqlx::query_as!(
        UserData,
        r#"SELECT id, home_time, home_lat, home_lon, home_display AS "home_display!",
                  work_time, work_lat, work_lon, work_display AS "work_display!", commute_minutes, alert_days, push_token, last_alerted_date
           FROM user_data WHERE id = $1"#,
        id,
    )
    .fetch_one(pool)
    .await
}

/// Upserts on `push_token` — re-onboarding on the same device (e.g. after
/// "start over", which only clears local app state) reuses the same Expo
/// push token, so this replaces that row instead of hitting the UNIQUE
/// constraint on a plain insert.
pub async fn create(pool: &Pool<Postgres>, user_data: &UserData) -> Result<UserData, sqlx::Error> {
    sqlx::query_as!(
        UserData,
        r#"INSERT INTO user_data
               (home_time, home_lat, home_lon, home_display, work_time, work_lat, work_lon, work_display, commute_minutes, alert_days, push_token)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           ON CONFLICT (push_token) DO UPDATE SET
               home_time = EXCLUDED.home_time,
               home_lat = EXCLUDED.home_lat,
               home_lon = EXCLUDED.home_lon,
               home_display = EXCLUDED.home_display,
               work_time = EXCLUDED.work_time,
               work_lat = EXCLUDED.work_lat,
               work_lon = EXCLUDED.work_lon,
               work_display = EXCLUDED.work_display,
               commute_minutes = EXCLUDED.commute_minutes,
               alert_days = EXCLUDED.alert_days,
               last_alerted_date = NULL
           RETURNING id, home_time, home_lat, home_lon, home_display AS "home_display!",
                     work_time, work_lat, work_lon, work_display AS "work_display!", commute_minutes, alert_days, push_token, last_alerted_date"#,
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
                     work_time, work_lat, work_lon, work_display AS "work_display!", commute_minutes, alert_days, push_token, last_alerted_date"#,
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

/// Records that a rain alert was already sent today, so `cron` won't send a second one.
pub async fn mark_alerted(
    pool: &Pool<Postgres>,
    id: Uuid,
    date: chrono::NaiveDate,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE user_data SET last_alerted_date = $1 WHERE id = $2",
        date,
        id,
    )
    .execute(pool)
    .await?;
    Ok(())
}
