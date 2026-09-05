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

        let spans = to_spans(wet, day_start, day_end);
        let spans = bridge(spans, GAP_TOLERANCE);
        let spans = cap(spans, MAX_INTERVALS);

        Ok(spans
            .into_iter()
            .map(|span| JacketInterval {
                from: span.from.with_timezone(&Oslo).time(),
                to: span.to.with_timezone(&Oslo).time(),
            })
            .collect())
    }

    /// Oslo-local midnight to midnight, as real instants.
    fn day_bounds(date: NaiveDate) -> (DateTime<Utc>, DateTime<Utc>) {
        let start = Self::time_window(date, NaiveTime::MIN, 0).0;
        let end = Self::time_window(date + Duration::days(1), NaiveTime::MIN, 0).0;
        (start, end)
    }
}

/// A dry spell this short between two showers isn't worth its own row — met.no is
/// hourly, so this is what collapses "shower, dry hour, shower" into one stretch.
const GAP_TOLERANCE: Duration = Duration::hours(1);

/// Past this many rows the screen stops being glanceable, so the narrowest gaps
/// get closed until the list fits — better to overstate a little rain than to
/// hand someone a timetable.
const MAX_INTERVALS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Span {
    from: DateTime<Utc>,
    to: DateTime<Utc>,
}

/// Clamps wet slots to the day and joins the ones that touch or overlap.
fn to_spans(wet: Vec<Slot>, day_start: DateTime<Utc>, day_end: DateTime<Utc>) -> Vec<Span> {
    let mut spans: Vec<Span> = Vec::new();

    for slot in wet {
        let span = Span {
            from: slot.start.max(day_start),
            to: slot.end.min(day_end),
        };
        match spans.last_mut() {
            Some(last) if span.from <= last.to => last.to = last.to.max(span.to),
            _ => spans.push(span),
        }
    }

    spans
}

/// Closes every dry gap at or under `tolerance`.
fn bridge(spans: Vec<Span>, tolerance: Duration) -> Vec<Span> {
    let mut merged: Vec<Span> = Vec::new();

    for span in spans {
        match merged.last_mut() {
            Some(last) if span.from - last.to <= tolerance => last.to = last.to.max(span.to),
            _ => merged.push(span),
        }
    }

    merged
}

/// Repeatedly swallows the narrowest remaining gap until at most `max` spans are left.
fn cap(mut spans: Vec<Span>, max: usize) -> Vec<Span> {
    while spans.len() > max {
        // `len > max >= 1` guarantees at least two spans, so this range is non-empty.
        let Some(next) = (1..spans.len()).min_by_key(|&i| spans[i].from - spans[i - 1].to) else {
            break;
        };
        spans[next - 1].to = spans[next].to;
        spans.remove(next);
    }

    spans
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
}

fn hhmm<S: serde::Serializer>(time: &NaiveTime, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&time.format("%H:%M").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    /// Spans as whole Oslo hours on an arbitrary summer day.
    fn spans(hours: &[(u32, u32)]) -> Vec<Span> {
        hours.iter().map(|&(from, to)| span(from, to)).collect()
    }

    fn span(from: u32, to: u32) -> Span {
        Span {
            from: at(from),
            to: at(to),
        }
    }

    fn at(hour: u32) -> DateTime<Utc> {
        format!("2026-09-07T{hour:02}:00:00+02:00")
            .parse()
            .unwrap()
    }

    fn hours(spans: &[Span]) -> Vec<(u32, u32)> {
        spans
            .iter()
            .map(|s| {
                (
                    s.from.with_timezone(&Oslo).hour(),
                    s.to.with_timezone(&Oslo).hour(),
                )
            })
            .collect()
    }

    #[test]
    fn bridge_closes_a_single_dry_hour() {
        let merged = bridge(spans(&[(6, 7), (8, 9)]), GAP_TOLERANCE);
        assert_eq!(hours(&merged), [(6, 9)]);
    }

    #[test]
    fn bridge_keeps_a_real_dry_stretch_apart() {
        let merged = bridge(spans(&[(6, 7), (12, 13)]), GAP_TOLERANCE);
        assert_eq!(hours(&merged), [(6, 7), (12, 13)]);
    }

    #[test]
    fn alternating_showers_collapse_to_one_stretch() {
        let every_other_hour = spans(&[(0, 1), (2, 3), (4, 5), (6, 7), (8, 9), (10, 11)]);
        let merged = bridge(every_other_hour, GAP_TOLERANCE);
        assert_eq!(hours(&merged), [(0, 11)]);
    }

    #[test]
    fn cap_closes_the_narrowest_gap_first() {
        // Gaps after bridging: 3h, 2h, 4h — the 2h one goes.
        let merged = cap(spans(&[(0, 1), (4, 5), (7, 8), (12, 13)]), 3);
        assert_eq!(hours(&merged), [(0, 1), (4, 8), (12, 13)]);
    }

    #[test]
    fn cap_leaves_a_short_list_alone() {
        let merged = cap(spans(&[(0, 1), (6, 7)]), MAX_INTERVALS);
        assert_eq!(hours(&merged), [(0, 1), (6, 7)]);
    }

    #[test]
    fn worst_case_day_fits_the_screen() {
        let scattered = spans(&[
            (0, 1),
            (3, 4),
            (6, 7),
            (9, 10),
            (12, 13),
            (15, 16),
            (18, 19),
            (21, 22),
        ]);
        let merged = cap(bridge(scattered, GAP_TOLERANCE), MAX_INTERVALS);
        assert!(merged.len() <= MAX_INTERVALS, "got {:?}", hours(&merged));
    }
}
