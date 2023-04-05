use crate::alias::DateTime;
use crate::portfolio::Way;

#[derive(Debug)]
pub struct Trade {
    pub date: DateTime,
    pub way: Way,
    pub quantity: f64,
    pub price: f64,
    pub tax: f64,
}
