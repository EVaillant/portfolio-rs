use chrono::{DateTime, Utc};

#[derive(Debug)]
pub enum CashVariationSource {
    Trade,
    Payement,
    Dividend,
}

#[derive(Debug)]
pub struct CashVariation {
    pub position: f64,
    pub date: DateTime<Utc>,
    pub source: CashVariationSource,
}
