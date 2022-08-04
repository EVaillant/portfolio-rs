mod cash_variation;
mod position;
mod trade;
mod way;

pub use cash_variation::*;
pub use position::*;
pub use trade::*;
pub use way::*;

use crate::marketdata::Currency;
use std::rc::Rc;

#[derive(Debug)]
pub struct Portfolio {
    pub name: String,
    pub currency: Rc<Currency>,
    pub positions: Vec<Position>,
    pub cash: Vec<CashVariation>,
}
