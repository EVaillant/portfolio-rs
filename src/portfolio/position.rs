use super::Trade;
use crate::portfolio::Way;
use crate::{alias::DateTime, marketdata::Instrument};
use std::rc::Rc;

#[derive(Debug)]
pub struct Position {
    pub instrument: Rc<Instrument>,
    pub trades: Vec<Trade>,
}

impl Position {
    pub fn get_close_date(&self) -> Option<DateTime> {
        let quantity: f64 = self
            .trades
            .iter()
            .map(|trade| trade.quantity * if trade.way == Way::Buy { -1.0 } else { 1.0 })
            .sum();
        if quantity.abs() < 1e-7 {
            self.trades.last().map(|trade| trade.date)
        } else {
            None
        }
    }
}
