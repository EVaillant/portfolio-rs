use super::Way;
use crate::alias::DateTime;

#[derive(Debug)]
pub struct Trade {
    pub date: DateTime,
    pub way: Way,
    pub quantity: f64,
    pub price: f64,
    pub fees: f64,
}
