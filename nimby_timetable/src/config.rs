use crate::types;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub now_time: types::Hms,
    pub now_week: i64,
}