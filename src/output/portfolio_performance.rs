use super::Output;
use crate::error::Error;
use crate::portfolio::{CashVariationSource, Portfolio, Way};
use core::panic;
use log::debug;
use std::collections::HashSet;

use std::fs::File;
use std::io::Write;

//
// doc https://help.portfolio-performance.info/en/reference/file/import/csv-import/
pub struct PortfolioPerformanceOutput<'a> {
    output_dir: String,
    portfolio: &'a Portfolio,
}

impl<'a> PortfolioPerformanceOutput<'a> {
    pub fn new(output_dir: &str, portfolio: &'a Portfolio) -> Self {
        let path = std::path::Path::new(&output_dir);
        if !path.is_dir() {
            panic!("{} must be a directory", output_dir);
        }
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

    fn write_instrument(&self) -> Result<(), Error> {
        let filename = format!("{}/{}_instrument.csv", self.output_dir, self.portfolio.name);
        let mut output_stream = File::create(filename)?;
        output_stream.write_all("Ticker Symbol;ISIN;Security Name;Currency\n".as_bytes())?;
        for instrument in self
            .portfolio
            .positions
            .iter()
            .map(|position| &position.instrument)
            .collect::<HashSet<_>>()
        {
            let mut buffer = String::new();
            if let Some(ticker) = instrument.ticker_yahoo.as_ref() {
                buffer.push_str(ticker);
            }
            buffer.push(';');
            buffer.push_str(instrument.isin.as_str());
            buffer.push(';');
            buffer.push_str(instrument.description.as_str());
            buffer.push(';');
            buffer.push_str(instrument.currency.name.as_str());
            buffer.push('\n');
            output_stream.write_all(buffer.as_bytes())?;
        }

        Ok(())
    }
}

impl Output for PortfolioPerformanceOutput<'_> {
    fn write(&mut self) -> Result<(), Error> {
        debug!("write instrument");
        self.write_instrument()?;

        debug!("write account");
        self.write_account()?;

        debug!("write trade");
        self.write_trade()?;

        Ok(())
    }
}
