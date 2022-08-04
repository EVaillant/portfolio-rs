use crate::marketdata::{Currency, Market};
use std::rc::Rc;
#[derive(Debug)]
pub struct Instrument {
    pub name: String,
    pub description: String,
    pub market: Rc<Market>,
    pub currency: Rc<Currency>,
    pub ticker_yahoo: Option<String>,
}
