use crate::error::Error;
use crate::output::Output;
use crate::portfolio::Portfolio;
use crate::pricer::PortfolioIndicators;

use std::fs::File;
use std::io::Write;

pub struct CsvOutput<'a> {
    output_dir: String,
    portfolio: &'a Portfolio,
    indicators: &'a PortfolioIndicators,
}

impl<'a> CsvOutput<'a> {
    pub fn new(
        output_dir: &str,
        portfolio: &'a Portfolio,
        indicators: &'a PortfolioIndicators,
    ) -> Self {
        Self {
            output_dir: output_dir.to_string(),
            portfolio,
            indicators,
        }
    }

    fn write_position_indicators(&self, filename: &str) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        output_stream.write_all(
            "Date;Cash;Valuation;Nominal;Dividends;Tax;P&L(%);P&L Daily(%);P&L Weekly(%),P&L Monthly(%);P&L Yearly(%);P&L;P&L Daily;P&L Weekly;P&L Monthly;P&L Yearly;Earning;Earning + Valuation\n".as_bytes(),
        )?;
        for portfolio_indicator in self.indicators.portfolios.iter() {
            output_stream.write_all(
                format!(
                    "{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}\n",
                    portfolio_indicator.date.format("%Y-%m-%d"),
                    portfolio_indicator.cash,
                    portfolio_indicator.valuation,
                    portfolio_indicator.nominal,
                    portfolio_indicator.dividends,
                    portfolio_indicator.tax,
                    portfolio_indicator.current_pnl.value_pct,
                    portfolio_indicator.daily_pnl.value_pct,
                    portfolio_indicator.weekly_pnl.value_pct,
                    portfolio_indicator.monthly_pnl.value_pct,
                    portfolio_indicator.yearly_pnl.value_pct,
                    portfolio_indicator.current_pnl.value,
                    portfolio_indicator.daily_pnl.value,
                    portfolio_indicator.weekly_pnl.value,
                    portfolio_indicator.monthly_pnl.value,
                    portfolio_indicator.yearly_pnl.value,
                    portfolio_indicator.earning,
                    portfolio_indicator.earning_latent
                )
                .as_bytes(),
            )?;
        }
        Ok(())
    }

    fn write_position_instrument_indicators(
        &self,
        instrument_name: &str,
        filename: &str,
    ) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        output_stream.write_all(
          "Date;Instrument;Spot(Close);Quantity;Unit Price;Valuation;Nominal;Dividends;Tax;P&L(%);P&L Daily(%);P&L Weekly(%);P&L Monthly(%);P&L Yearly(%);P&L;P&L Daily;P&L Weekly;P&L Monthly;P&L Yearly;Earning;Earning + Valuation\n".as_bytes(),
      )?;
        for portfolio_indicator in self.indicators.portfolios.iter() {
            if let Some(position_indicator) = portfolio_indicator
                .positions
                .iter()
                .find(|item| item.instrument.name == instrument_name)
            {
                output_stream.write_all(
                    format!(
                        "{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}\n",
                        portfolio_indicator.date.format("%Y-%m-%d"),
                        instrument_name,
                        position_indicator.spot.close(),
                        position_indicator.quantity,
                        position_indicator.unit_price,
                        position_indicator.valuation,
                        position_indicator.nominal,
                        position_indicator.dividends,
                        position_indicator.tax,
                        position_indicator.current_pnl.value_pct,
                        position_indicator.daily_pnl.value_pct,
                        position_indicator.weekly_pnl.value_pct,
                        position_indicator.monthly_pnl.value_pct,
                        position_indicator.yearly_pnl.value_pct,
                        position_indicator.current_pnl.value,
                        position_indicator.daily_pnl.value,
                        position_indicator.weekly_pnl.value,
                        position_indicator.monthly_pnl.value,
                        position_indicator.yearly_pnl.value,
                        position_indicator.earning,
                        position_indicator.earning_latent,
                    )
                    .as_bytes(),
                )?;
            }
        }
        Ok(())
    }
}

impl<'a> Output for CsvOutput<'a> {
    fn write_indicators(&mut self) -> Result<(), Error> {
        let filename = format!("{}/indicators_{}.csv", self.output_dir, self.portfolio.name);
        self.write_position_indicators(&filename)?;

        for instrument_name in self.portfolio.get_instrument_name_list().iter() {
            let filename = format!(
                "{}/indicators_{}_{}.csv",
                self.output_dir, self.portfolio.name, instrument_name
            );
            self.write_position_instrument_indicators(instrument_name, &filename)?;
        }

        Ok(())
    }
}
