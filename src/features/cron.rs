use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{Datelike, Duration, Local};

use crate::features::clients::weather_client::{Weather, WeatherClient};
use crate::features::db::{UserData, user_data as user_data_db};
use crate::state::AppState;

const TICK_INTERVAL: StdDuration = StdDuration::from_secs(15 * 60);

enum Leg {
    Home,
    Work,
}

/// Runs forever: every 15 minutes, alerts anyone whose home/work departure
/// falls in the next 15-minute window with a weather-based push notification.
pub async fn spawn(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(TICK_INTERVAL);
    loop {
        interval.tick().await;
        run_once(&state).await;
    }
}

async fn run_once(state: &Arc<AppState>) {
    let users = match user_data_db::get_all_with_push_token(state.get_pool()).await {
        Ok(users) => users,
        Err(error) => {
            eprintln!("cron: failed to load user data: {error}");
            return;
        }
    };

    let now = Local::now();
    let weekday = now.weekday().num_days_from_monday() as usize;
    let window_start = (now + Duration::minutes(15)).time();
    let window_end = (now + Duration::minutes(30)).time();

    for user in users {
        if user.alert_days.get(weekday) != Some(true) {
            continue;
        }

        if WeatherClient::in_range(user.home_time, window_start, window_end) {
            notify(state, &user, Leg::Home).await;
        }
        if WeatherClient::in_range(user.work_time, window_start, window_end) {
            notify(state, &user, Leg::Work).await;
        }
    }
}

async fn notify(state: &Arc<AppState>, user: &UserData, leg: Leg) {
    let Some(token) = user.push_token.clone() else {
        return;
    };
    let (lat, lon) = match leg {
        Leg::Home => (user.home_lat, user.home_lon),
        Leg::Work => (user.work_lat, user.work_lon),
    };

    let weather = match state
        .get_weather_client()
        .get_weather(user.home_time, user.work_time, lat, lon)
        .await
    {
        Ok(weather) => weather,
        Err(error) => {
            eprintln!("cron: failed to fetch weather for {}: {error}", user.id);
            return;
        }
    };

    let (title, body) = copy_for(weather);
    if let Err(error) = state.get_expo_push_client().send(&[token], title, body).await {
        eprintln!("cron: failed to send push for {}: {error}", user.id);
    }
}

fn copy_for(weather: Weather) -> (&'static str, &'static str) {
    match weather {
        Weather::Rainy => ("Ta med regnjakka", "Regn er ventet på turen din."),
        Weather::Sunny => ("La regnjakka ligge hjemme", "Tørt vær på turen din."),
        Weather::Cloudy => ("Jakke ja, regnjakke nei", "Overskyet og tørt på turen din."),
    }
}
