#[path = "../src/clients/mod.rs"]
mod clients;

use chrono::{NaiveTime, Utc};
use chrono_tz::Europe::Oslo;
use clients::weather_client::WeatherClient;

const OSLO_LAT: f64 = 59.9139;
const OSLO_LON: f64 = 10.7522;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("weather_client=debug")),
        )
        .init();

    let client = WeatherClient::new(reqwest::Client::new());

    let home_time = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
    let work_time = NaiveTime::from_hms_opt(16, 0, 0).unwrap();

    let today = Utc::now().with_timezone(&Oslo).date_naive();

    let weather = client
        .get_weather(
            today, home_time, OSLO_LAT, OSLO_LON, work_time, OSLO_LAT, OSLO_LON, 30,
        )
        .await
        .expect("Failed to fetch weather");

    println!("commute {today}: {weather:?}");

    let intervals = client
        .jacket_intervals(today, OSLO_LAT, OSLO_LON)
        .await
        .expect("Failed to fetch jacket intervals");

    println!("jacket intervals {today}: {intervals:?}");
}
