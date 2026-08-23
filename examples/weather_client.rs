#[path = "../src/features/clients/mod.rs"]
mod clients;

use chrono::NaiveTime;
use clients::weather_client::WeatherClient;

const OSLO_LAT: f64 = 59.9139;
const OSLO_LON: f64 = 10.7522;

#[tokio::main]
async fn main() {
    let client = WeatherClient::new(reqwest::Client::new());

    let home = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
    let work = NaiveTime::from_hms_opt(16, 0, 0).unwrap();

    let weather = client
        .get_weather(home, work, OSLO_LAT, OSLO_LON)
        .await
        .expect("Failed to fetch weather");

    println!("{weather:?}");
}
