pub mod setup;
pub mod user_data;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// `bit_vec::BitVec` only derives serde for its raw internal representation
/// (storage words + bit count), which isn't something a JSON client should
/// have to construct. These (de)serialize it as a plain `bool` array instead,
/// one entry per day, Mon..Sun.
mod alert_days_serde {
    use bit_vec::BitVec;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bits: &BitVec, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        bits.iter().collect::<Vec<bool>>().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BitVec, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bools = Vec::<bool>::deserialize(deserializer)?;
        Ok(BitVec::from_iter(bools))
    }

    pub mod option {
        use bit_vec::BitVec;
        use serde::{Deserialize, Deserializer};

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<BitVec>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let bools = Option::<Vec<bool>>::deserialize(deserializer)?;
            Ok(bools.map(BitVec::from_iter))
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct UserData {
    #[serde(default)]
    pub id: Uuid,
    pub home_time: chrono::NaiveTime,
    pub home_lat: f64,
    pub home_lon: f64,
    pub home_display: String,
    pub work_time: chrono::NaiveTime,
    pub work_lat: f64,
    pub work_lon: f64,
    pub work_display: String,
    pub commute_minutes: i32,
    #[serde(with = "alert_days_serde")]
    pub alert_days: bit_vec::BitVec,
    pub push_token: Option<String>,
    #[serde(default)]
    pub last_alerted_date: Option<chrono::NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct UserDataPatchRequest {
    pub home_time: Option<chrono::NaiveTime>,
    pub home_lat: Option<f64>,
    pub home_lon: Option<f64>,
    pub home_display: Option<String>,
    pub work_time: Option<chrono::NaiveTime>,
    pub work_lat: Option<f64>,
    pub work_lon: Option<f64>,
    pub work_display: Option<String>,
    pub commute_minutes: Option<i32>,
    #[serde(default, with = "alert_days_serde::option")]
    pub alert_days: Option<bit_vec::BitVec>,
    pub push_token: Option<String>,
}
