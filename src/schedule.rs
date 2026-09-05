//! Users enter home/work times as Oslo wall-clock, so every date/time decision
//! must be made in Oslo — not the host's timezone (UTC in Docker).

use chrono::{DateTime, Utc};
use chrono_tz::Europe::Oslo;
use chrono_tz::Tz;

pub fn now_oslo() -> DateTime<Tz> {
    Utc::now().with_timezone(&Oslo)
}
