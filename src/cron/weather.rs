use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{Datelike, Duration, NaiveDate, Timelike};

use crate::clients::weather_client::{Weather, WeatherClient};
use crate::db::{UserData, user_data as user_data_db};
use crate::schedule::now_oslo;
use crate::state::AppState;

const TICK_INTERVAL: StdDuration = StdDuration::from_secs(5 * 60);
const LOOKAHEAD: Duration = Duration::minutes(15);

/// Runs forever: every 5 minutes, on the round clock mark, alerts anyone
/// whose home/work departure is exactly 15 minutes out — but only if it's
/// rainy, and only once per user per day.
pub async fn spawn(state: Arc<AppState>) {
    align_to_tick().await;
    let mut interval = tokio::time::interval(TICK_INTERVAL);
    loop {
        interval.tick().await;
        run_once(&state).await;
    }
}

/// Sleeps until the next round 5-minute wall-clock mark (:00, :05, :10, ...),
/// so ticks land on times like 06:45 rather than wherever the process booted.
async fn align_to_tick() {
    let now = now_oslo();
    let interval_secs = TICK_INTERVAL.as_secs() as i64;
    let secs_into_hour = (now.minute() as i64) * 60 + now.second() as i64;
    let remainder = secs_into_hour % interval_secs;
    let delay = if remainder == 0 {
        0
    } else {
        interval_secs - remainder
    };
    tokio::time::sleep(StdDuration::from_secs(delay as u64)).await;
}

async fn run_once(state: &Arc<AppState>) {
    tracing::trace!("cron: tick");

    let users = match user_data_db::get_all_with_push_token(state.get_pool()).await {
        Ok(users) => users,
        Err(error) => {
            tracing::error!(%error, "cron: failed to load user data");
            return;
        }
    };
    let now = now_oslo();
    let today = now.date_naive();
    let weekday = now.weekday().num_days_from_monday() as usize;
    // Floor to the tick mark so window bounds are deterministic, not shifted by
    // the few ms of jitter between the tick firing and now_oslo() being sampled.
    let tick_mark = now
        - Duration::seconds(now.second() as i64)
        - Duration::nanoseconds(now.nanosecond() as i64);
    let window_start = (tick_mark + LOOKAHEAD).time();
    let window_end = (tick_mark + LOOKAHEAD + Duration::minutes(5)).time();
    tracing::info!(count = users.len(), %weekday, %window_start, %window_end, "cron: checking users against alert window");

    for user in users {
        if user.alert_days.get(weekday) != Some(true) {
            tracing::trace!(user_id = %user.id, "cron: not alerting today, skipping");
            continue;
        }
        if user.last_alerted_date == Some(today) {
            tracing::trace!(user_id = %user.id, "cron: already alerted today, skipping");
            continue;
        }

        let home_due = WeatherClient::in_range(user.home_time, window_start, window_end);
        let work_due = WeatherClient::in_range(user.work_time, window_start, window_end);
        if !home_due && !work_due {
            continue;
        }

        tracing::info!(user_id = %user.id, home_due, work_due, "cron: user due in alert window");
        maybe_notify(state, &user, today).await;
    }
}

/// Sends a rain alert if — and only if — it's actually going to rain on the
/// commute. Marks the user as alerted for `today` on success, so this fires
/// at most once per day per user.
async fn maybe_notify(state: &Arc<AppState>, user: &UserData, today: NaiveDate) {
    let Some(token) = user.push_token.clone() else {
        return;
    };
    let weather = match state
        .get_weather_client()
        .get_weather(
            today,
            user.home_time,
            user.home_lat,
            user.home_lon,
            user.work_time,
            user.work_lat,
            user.work_lon,
            user.commute_minutes,
        )
        .await
    {
        Ok(weather) => weather,
        Err(error) => {
            tracing::error!(user_id = %user.id, %error, "cron: failed to fetch weather");
            return;
        }
    };
    tracing::trace!(user_id = %user.id, ?weather, "cron: weather fetched");

    if weather != Weather::Rainy {
        tracing::trace!(user_id = %user.id, "cron: no rain, not alerting");
        return;
    }

    tracing::info!(user_id = %user.id, "cron: sending push notification");
    match state
        .get_expo_push_client()
        .send(&[token], "Ta med regnjakka", "Regn er ventet på turen din.")
        .await
    {
        Ok(response) => {
            tracing::info!(user_id = %user.id, %response, "cron: push sent");
            if let Err(error) = user_data_db::mark_alerted(state.get_pool(), user.id, today).await {
                tracing::error!(user_id = %user.id, %error, "cron: failed to record alert");
            }
        }
        Err(error) => tracing::error!(user_id = %user.id, %error, "cron: failed to send push"),
    }
}
