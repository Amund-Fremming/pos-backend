use chrono::{Duration, NaiveTime};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "pos-backend/0.1.0 github.com/amundfremming/pos-backend";
const LOCATIONFORECAST_URL: &str = "https://api.met.no/weatherapi/locationforecast/2.0/compact";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Weather {
    #[serde(rename = "rain")]
    Rainy,
    Cloudy,
    #[serde(rename = "sun")]
    Sunny,
}

#[derive(Deserialize)]
struct ForecastResponse {
    properties: Properties,
}

#[derive(Deserialize)]
struct Properties {
    timeseries: Vec<TimeseriesEntry>,
}

#[derive(Deserialize)]
struct TimeseriesEntry {
    time: chrono::DateTime<chrono::Utc>,
    data: TimeseriesData,
}

#[derive(Deserialize)]
struct TimeseriesData {
    next_1_hours: Option<NextPeriod>,
    next_6_hours: Option<NextPeriod>,
}

#[derive(Deserialize)]
struct NextPeriod {
    summary: Summary,
}

#[derive(Deserialize)]
struct Summary {
    symbol_code: String,
}

#[derive(Clone)]
pub struct WeatherClient {
    client: reqwest::Client,
}

impl WeatherClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    fn time_window(anchor: NaiveTime) -> (NaiveTime, NaiveTime) {
        let start = anchor - Duration::minutes(30);
        let end = anchor + Duration::hours(1);
        (start, end)
    }

    pub(crate) fn in_range(time: NaiveTime, start: NaiveTime, end: NaiveTime) -> bool {
        if start <= end {
            return time >= start && time <= end;
        }
        time >= start || time <= end
    }

    fn classify(symbol_codes: &[String]) -> Weather {
        if symbol_codes
            .iter()
            .any(|code| code.contains("rain") || code.contains("sleet") || code.contains("snow"))
        {
            return Weather::Rainy;
        }
        let sunny = symbol_codes
            .iter()
            .filter(|code| code.contains("clearsky") || code.contains("fair"))
            .count();
        let cloudy = symbol_codes
            .iter()
            .filter(|code| code.contains("cloudy") || code.contains("fog"))
            .count();

        if sunny > cloudy {
            Weather::Sunny
        } else {
            Weather::Cloudy
        }
    }

    pub async fn get_weather(
        &self,
        home: NaiveTime,
        work: NaiveTime,
        lat: f64,
        lon: f64,
    ) -> Result<Weather, reqwest::Error> {
        let response = self
            .client
            .get(LOCATIONFORECAST_URL)
            .header("User-Agent", USER_AGENT)
            .query(&[("lat", lat.to_string()), ("lon", lon.to_string())])
            .send()
            .await?
            .json::<ForecastResponse>()
            .await?;

        let windows = [Self::time_window(home), Self::time_window(work)];

        let symbol_codes: Vec<String> = response
            .properties
            .timeseries
            .into_iter()
            .filter(|entry| {
                let time = entry.time.time();
                windows
                    .iter()
                    .any(|(start, end)| Self::in_range(time, *start, *end))
            })
            .filter_map(|entry| {
                entry
                    .data
                    .next_1_hours
                    .or(entry.data.next_6_hours)
                    .map(|period| period.summary.symbol_code)
            })
            .collect();

        Ok(Self::classify(&symbol_codes))
    }
}
