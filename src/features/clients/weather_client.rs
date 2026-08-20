const USER_AGENT: &str = "pos-backend/0.1.0 github.com/amundfremming/pos-backend";
const LOCATIONFORECAST_URL: &str = "https://api.met.no/weatherapi/locationforecast/2.0/compact";

pub struct WeatherClient {
    client: reqwest::Client,
}

impl WeatherClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub async fn get_weather(
        &self,
        hour: u8,
        minute: u8,
        lat: f64,
        lon: f64,
    ) -> Result<serde_json::Value, reqwest::Error> {
        let response = self
            .client
            .get(LOCATIONFORECAST_URL)
            .header("User-Agent", USER_AGENT)
            .query(&[("lat", lat.to_string()), ("lon", lon.to_string())])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        Ok(response)
    }
}
