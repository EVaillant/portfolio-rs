use crate::portfolio::Way;
use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct Trade {
    pub date: DateTime<Utc>,
    pub way: Way,
    pub quantity: u32,
    pub price: f64,
    pub tax: f64,
}
