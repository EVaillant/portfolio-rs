use crate::alias::DateTime;

#[derive(Debug, PartialEq)]
pub enum CashVariationSource {
    Payment,
}

#[derive(Debug)]
pub struct CashVariation {
    pub position: f64,
    pub date: DateTime,
    pub source: CashVariationSource,
}
