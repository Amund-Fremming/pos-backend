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

    /// Precise commute window: departure time through departure + commute duration, no padding either side.
    fn time_window(departure: NaiveTime, commute_minutes: i32) -> (NaiveTime, NaiveTime) {
        let end = departure + Duration::minutes(commute_minutes as i64);
        (departure, end)
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
            tracing::debug!(?symbol_codes, "classified as Rainy");
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

        let weather = if sunny > cloudy {
            Weather::Sunny
        } else {
            Weather::Cloudy
        };
        tracing::debug!(?symbol_codes, sunny, cloudy, ?weather, "classified");
        weather
    }

    async fn symbol_codes_in_window(
        &self,
        location: &str,
        lat: f64,
        lon: f64,
        start: NaiveTime,
        end: NaiveTime,
    ) -> Result<Vec<String>, reqwest::Error> {
        let response = self
            .client
            .get(LOCATIONFORECAST_URL)
            .header("User-Agent", USER_AGENT)
            .query(&[("lat", lat.to_string()), ("lon", lon.to_string())])
            .send()
            .await?
            .json::<ForecastResponse>()
            .await?;

        let entries: Vec<(chrono::DateTime<chrono::Utc>, String)> = response
            .properties
            .timeseries
            .into_iter()
            .filter(|entry| Self::in_range(entry.time.time(), start, end))
            .filter_map(|entry| {
                let time = entry.time;
                entry
                    .data
                    .next_1_hours
                    .or(entry.data.next_6_hours)
                    .map(|period| (time, period.summary.symbol_code))
            })
            .collect();

        for (time, code) in &entries {
            tracing::debug!("{time} ({location}): {code}");
        }

        Ok(entries.into_iter().map(|(_, code)| code).collect())
    }

    /// Checks weather across both commute legs (home->work and work->home), covering
    /// both the departure and arrival locations over each precise departure -> departure+commute window.
    /// Combined because what matters is whether rain shows up in any of these intervals.
    pub async fn get_weather(
        &self,
        home_time: NaiveTime,
        home_lat: f64,
        home_lon: f64,
        work_time: NaiveTime,
        work_lat: f64,
        work_lon: f64,
        commute_minutes: i32,
    ) -> Result<Weather, reqwest::Error> {
        let mut symbol_codes = Vec::new();

        for departure in [home_time, work_time] {
            let (start, end) = Self::time_window(departure, commute_minutes);
            symbol_codes.extend(
                self.symbol_codes_in_window("home", home_lat, home_lon, start, end)
                    .await?,
            );
            symbol_codes.extend(
                self.symbol_codes_in_window("work", work_lat, work_lon, start, end)
                    .await?,
            );
        }

        Ok(Self::classify(&symbol_codes))
    }
}
