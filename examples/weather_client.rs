#[path = "../src/features/clients/mod.rs"]
mod clients;

use clients::weather_client::WeatherClient;

const OSLO_LAT: f64 = 59.9139;
const OSLO_LON: f64 = 10.7522;

#[tokio::main]
async fn main() {
    let client = WeatherClient::new(reqwest::Client::new());

    let weather = client
        .get_weather(7, 30, OSLO_LAT, OSLO_LON)
        .await
        .expect("Failed to fetch weather");

    println!("{weather:#}");
}
