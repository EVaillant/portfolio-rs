use crate::alias::DateTime;
use crate::marketdata::{Currency, Market};
use std::rc::Rc;

#[derive(Debug)]
#[allow(dead_code)]
pub struct Instrument {
    pub name: String,
    pub isin: String,
    pub description: String,
    pub market: Rc<Market>,
    pub currency: Rc<Currency>,
    pub ticker_yahoo: Option<String>,
    pub region: Option<String>,
    pub fund_category: String,
    pub dividends: Option<Vec<Dividend>>,
}

#[derive(Debug)]
pub struct Dividend {
    pub record_date: DateTime,
    pub payment_date: DateTime,
    pub value: f64,
}

impl std::hash::Hash for Instrument {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.name.hash(state)
    }
}

impl std::cmp::PartialEq for Instrument {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl std::cmp::Eq for Instrument {}
