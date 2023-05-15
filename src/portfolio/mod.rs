mod cash_variation;
mod position;
mod trade;
mod way;

pub use cash_variation::*;
pub use position::*;
pub use trade::*;
pub use way::*;

use crate::alias::Date;
use crate::error::Error;
use crate::marketdata::Currency;
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Debug)]
pub struct Portfolio {
    pub name: String,
    pub currency: Rc<Currency>,
    pub positions: Vec<Position>,
    pub cash: Vec<CashVariation>,
}

impl Portfolio {
    pub fn get_trade_date(&self) -> Result<Date, Error> {
        let mut trade_dates = self
            .positions
            .iter()
            .flat_map(|position| position.trades.first())
            .map(|trade| trade.date)
            .collect::<Vec<_>>();
        trade_dates.sort();

        let first_trade = trade_dates.first().ok_or(Error::new_portfolio(
            "unable to detect first trade date in the portfolio",
        ))?;

        Ok(first_trade.date())
    }

    pub fn get_instrument_name_list(&self) -> HashSet<&String> {
        self.positions
            .iter()
            .map(|position| &position.instrument.name)
            .collect()
    }

    pub fn get_region_name_list(&self) -> HashSet<&String> {
        self.positions
            .iter()
            .map(|position| &position.instrument.region)
            .collect()
    }
}
