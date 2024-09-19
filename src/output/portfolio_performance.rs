use super::Output;
use crate::error::Error;
use crate::portfolio::{CashVariationSource, Portfolio, Way};
use log::debug;

use std::fs::File;
use std::io::Write;

pub struct PortfolioPerformanceOutput<'a> {
    output_dir: String,
    portfolio: &'a Portfolio,
}

impl<'a> PortfolioPerformanceOutput<'a> {
    pub fn new(output_dir: &str, portfolio: &'a Portfolio) -> Self {
        Self {
            output_dir: output_dir.to_string(),
            portfolio,
        }
    }

    fn write_account(&self) -> Result<(), Error> {
        let filename = format!("{}/{}_account.csv", self.output_dir, self.portfolio.name);
        let mut output_stream = File::create(filename)?;
        output_stream.write_all("Date;Value\n".as_bytes())?;
        for cash in self
            .portfolio
            .cash
            .iter()
            .filter(|item| item.source == CashVariationSource::Payment)
        {
            output_stream.write_all(
                format!("{};{}\n", cash.date.format("%Y-%m-%d"), cash.position,).as_bytes(),
            )?;
        }
        Ok(())
    }

    fn write_trade(&self) -> Result<(), Error> {
        let filename = format!("{}/{}_trade.csv", self.output_dir, self.portfolio.name);
        let mut output_stream = File::create(filename)?;
        output_stream.write_all("Date;Way;Isin;Quantity;Price;Fees\n".as_bytes())?;
        for (instrument, trade) in self.portfolio.positions.iter().flat_map(|position| {
            position
                .trades
                .iter()
                .map(|trade| (&position.instrument, trade))
        }) {
            output_stream.write_all(
                format!(
                    "{};{};{};{};{};{}\n",
                    trade.date.format("%Y-%m-%d"),
                    if trade.way == Way::Buy { "Buy" } else { "Sell" },
                    instrument.isin,
                    trade.quantity,
                    trade.price * trade.quantity + trade.fees,
                    trade.fees
                )
                .as_bytes(),
            )?;
        }
        Ok(())
    }
}

impl<'a> Output for PortfolioPerformanceOutput<'a> {
    fn write(&mut self) -> Result<(), Error> {
        debug!("write account");
        self.write_account()?;

        debug!("write trade");
        self.write_trade()?;

        Ok(())
    }
}
