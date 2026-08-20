pub mod commute;
pub mod setup;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Commute {
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
    pub alert_days: bit_vec::BitVec,
}

#[derive(Debug, Deserialize)]
pub struct CommutePatchRequest {
    pub home_time: Option<chrono::NaiveTime>,
    pub home_lat: Option<f64>,
    pub home_lon: Option<f64>,
    pub home_display: Option<String>,
    pub work_time: Option<chrono::NaiveTime>,
    pub work_lat: Option<f64>,
    pub work_lon: Option<f64>,
    pub work_display: Option<String>,
    pub alert_days: Option<bit_vec::BitVec>,
}
