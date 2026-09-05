use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use chrono_tz::Europe::Oslo;
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

impl TimeseriesData {
    /// The forecast symbol plus how long it covers. met.no drops from hourly to
    /// 6-hour steps a few days out, so the coverage has to travel with the code.
    fn symbol_and_coverage(self) -> Option<(String, Duration)> {
        if let Some(period) = self.next_1_hours {
            return Some((period.summary.symbol_code, Duration::hours(1)));
        }
        let period = self.next_6_hours?;
        Some((period.summary.symbol_code, Duration::hours(6)))
    }
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

    /// Precise commute window on `date`: departure through departure + commute duration,
    /// no padding either side. Departures are Oslo wall-clock, resolved to real instants
    /// so they can be compared against the forecast's UTC timestamps.
    fn time_window(
        date: NaiveDate,
        departure: NaiveTime,
        commute_minutes: i32,
    ) -> (DateTime<Utc>, DateTime<Utc>) {
        let local = date.and_time(departure);
        // DST gap/overlap: either side of the jump is close enough for a commute window.
        let start = local
            .and_local_timezone(Oslo)
            .earliest()
            .map(|start| start.with_timezone(&Utc))
            .unwrap_or_else(|| local.and_utc());
        (start, start + Duration::minutes(commute_minutes as i64))
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

    async fn forecast(&self, lat: f64, lon: f64) -> Result<Vec<Slot>, reqwest::Error> {
        let response = self
            .client
            .get(LOCATIONFORECAST_URL)
            .header("User-Agent", USER_AGENT)
            .query(&[("lat", lat.to_string()), ("lon", lon.to_string())])
            .send()
            .await?
            .json::<ForecastResponse>()
            .await?;

        Ok(response
            .properties
            .timeseries
            .into_iter()
            .filter_map(|entry| {
                let start = entry.time;
                entry
                    .data
                    .symbol_and_coverage()
                    .map(|(symbol_code, coverage)| Slot {
                        start,
                        end: start + coverage,
                        symbol_code,
                    })
            })
            .collect())
    }

    async fn symbol_codes_in_window(
        &self,
        location: &str,
        lat: f64,
        lon: f64,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<String>, reqwest::Error> {
        let slots: Vec<Slot> = self
            .forecast(lat, lon)
            .await?
            .into_iter()
            .filter(|slot| slot.overlaps(start, end))
            .collect();

        for slot in &slots {
            tracing::debug!("{} ({location}): {}", slot.start, slot.symbol_code);
        }

        Ok(slots.into_iter().map(|slot| slot.symbol_code).collect())
    }

    /// Checks weather across both commute legs (home->work and work->home) on `date`,
    /// covering both the departure and arrival locations over each precise
    /// departure -> departure+commute window. Combined because what matters is whether
    /// rain shows up in any of these intervals.
    pub async fn get_weather(
        &self,
        date: NaiveDate,
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
            let (start, end) = Self::time_window(date, departure, commute_minutes);
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

    /// Off-day view: the stretches of `date` you'd want a jacket for, at a single
    /// location. Not a general forecast — only wet intervals, merged so an
    /// all-morning drizzle reads as one row rather than six.
    pub async fn jacket_intervals(
        &self,
        date: NaiveDate,
        lat: f64,
        lon: f64,
    ) -> Result<Vec<JacketInterval>, reqwest::Error> {
        let (day_start, day_end) = Self::day_bounds(date);

        let wet: Vec<Slot> = self
            .forecast(lat, lon)
            .await?
            .into_iter()
            .filter(|slot| slot.is_wet() && slot.overlaps(day_start, day_end))
            .collect();

        Ok(Self::merge(wet, day_start, day_end))
    }

    /// Oslo-local midnight to midnight, as real instants.
    fn day_bounds(date: NaiveDate) -> (DateTime<Utc>, DateTime<Utc>) {
        let start = Self::time_window(date, NaiveTime::MIN, 0).0;
        let end = Self::time_window(date + Duration::days(1), NaiveTime::MIN, 0).0;
        (start, end)
    }

    /// Collapses touching or overlapping wet slots into single intervals, clamped
    /// to the day and reported as Oslo wall-clock.
    fn merge(
        wet: Vec<Slot>,
        day_start: DateTime<Utc>,
        day_end: DateTime<Utc>,
    ) -> Vec<JacketInterval> {
        let mut merged: Vec<JacketInterval> = Vec::new();

        for slot in wet {
            let from = slot.start.max(day_start);
            let to = slot.end.min(day_end);

            if let Some(last) = merged.last_mut()
                && from <= last.to_utc
            {
                if to > last.to_utc {
                    last.to_utc = to;
                    last.to = to.with_timezone(&Oslo).time();
                }
                continue;
            }

            merged.push(JacketInterval {
                from: from.with_timezone(&Oslo).time(),
                to: to.with_timezone(&Oslo).time(),
                to_utc: to,
            });
        }

        merged
    }
}

struct Slot {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    symbol_code: String,
}

impl Slot {
    /// Half-open on both sides so a slot merely touching a boundary doesn't count.
    fn overlaps(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> bool {
        self.start < end && self.end > start
    }

    fn is_wet(&self) -> bool {
        let code = &self.symbol_code;
        code.contains("rain") || code.contains("sleet") || code.contains("snow")
    }
}

/// A stretch of the day worth carrying a jacket for, in Oslo wall-clock.
#[derive(Debug, Clone, Serialize)]
pub struct JacketInterval {
    #[serde(serialize_with = "hhmm")]
    pub from: NaiveTime,
    #[serde(serialize_with = "hhmm")]
    pub to: NaiveTime,
    /// Merge bookkeeping — `to` alone can't order across midnight.
    #[serde(skip)]
    to_utc: DateTime<Utc>,
}

fn hhmm<S: serde::Serializer>(time: &NaiveTime, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&time.format("%H:%M").to_string())
}
