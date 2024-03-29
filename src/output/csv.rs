use super::Output;
use crate::alias::Date;
use crate::error::Error;
use crate::portfolio::Portfolio;
use crate::pricer::{HeatMapItem, PortfolioIndicators};

use std::fs::File;
use std::io::Write;

fn convert_to_cvs(year: i32, item: &HeatMapItem) -> String {
    let str_line = item
        .data()
        .iter()
        .map(|item| {
            if let Some(v) = item {
                (v * 100.0).to_string()
            } else {
                "".to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(";");
    format!("{};{}\n", year, str_line)
}

pub struct CsvOutput<'a> {
    output_dir: String,
    portfolio: &'a Portfolio,
    indicators: &'a PortfolioIndicators,
    filter_indicators: &'a Option<Date>,
}

impl<'a> CsvOutput<'a> {
    pub fn new(
        output_dir: &str,
        portfolio: &'a Portfolio,
        indicators: &'a PortfolioIndicators,
        filter_indicators: &'a Option<Date>,
    ) -> Self {
        Self {
            output_dir: output_dir.to_string(),
            portfolio,
            indicators,
            filter_indicators,
        }
    }

    fn write_distribution_by_region(&self, filename: &str) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        if let Some(portfolio) = self.indicators.portfolios.last() {
            let data = portfolio.make_distribution_by_region();
            for (region_name, pct) in data {
                output_stream.write_all(format!("{};{}\n", region_name, pct).as_bytes())?;
            }
        }
        Ok(())
    }

    fn write_distribution_global_by_instrument(&self, filename: &str) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        if let Some(portfolio) = self.indicators.portfolios.last() {
            let data = portfolio.make_distribution_global_by_instrument();
            for (region_name, pct) in data {
                output_stream.write_all(format!("{};{}\n", region_name, pct).as_bytes())?;
            }
        }
        Ok(())
    }

    fn write_distribution_by_instrument(
        &self,
        region_name: &str,
        filename: &str,
    ) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        if let Some(portfolio) = self.indicators.portfolios.last() {
            let data = portfolio.make_distribution_by_instrument(region_name);
            for (region_name, pct) in data {
                output_stream.write_all(format!("{};{}\n", region_name, pct).as_bytes())?;
            }
        }
        Ok(())
    }

    fn write_instrument_heat_map(
        &self,
        instrument_name: &str,
        filename: &str,
    ) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        output_stream
            .write_all("Year;Jan;Feb;Mar;Apr;May;Jun;Jul;Aug,Sep;Oct;Nov;Dec\n".as_bytes())?;

        for line in self
            .indicators
            .make_month_instrument_heat_map(instrument_name)
            .iter()
            .map(|(year, item)| convert_to_cvs(*year, item))
        {
            output_stream.write_all(line.as_bytes())?;
        }

        Ok(())
    }

    fn write_heat_map(&self, filename: &str) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        output_stream
            .write_all("Year;Jan;Feb;Mar;Apr;May;Jun;Jul;Aug,Sep;Oct;Nov;Dec\n".as_bytes())?;

        for line in self
            .indicators
            .make_month_heat_map()
            .iter()
            .map(|(year, item)| convert_to_cvs(*year, item))
        {
            output_stream.write_all(line.as_bytes())?;
        }

        Ok(())
    }

    fn write_position_indicators(&self, filename: &str) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        output_stream.write_all(
            "Date;Cash;Incoming Transfert;Outcoming Transfert;Valuation;Nominal;Dividends;Tax;P&L(%);P&L Daily(%);P&L Weekly(%),P&L Monthly(%);P&L Yearly(%);P&L for 3 Months(%);P&L for one Year(%);P&L;P&L Daily;P&L Weekly;P&L Monthly;P&L Yearly;P&L for 3 Months;P&L for one Year;Volatility 3M;Volatility 1Y;Earning;Earning + Valuation\n".as_bytes(),
        )?;
        let mut have_line = false;
        for portfolio_indicator in self.indicators.portfolios.iter() {
            if self
                .filter_indicators
                .map_or(false, |date| date > portfolio_indicator.date)
            {
                continue;
            }
            have_line = true;
            output_stream.write_all(
                format!(
                    "{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}\n",
                    portfolio_indicator.date.format("%Y-%m-%d"),
                    portfolio_indicator.cash,
                    portfolio_indicator.incoming_transfer,
                    portfolio_indicator.outcoming_transfer,
                    portfolio_indicator.valuation,
                    portfolio_indicator.nominal,
                    portfolio_indicator.dividends,
                    portfolio_indicator.tax,
                    portfolio_indicator.pnl_current.value_pct,
                    portfolio_indicator.pnl_daily.value_pct,
                    portfolio_indicator.pnl_weekly.value_pct,
                    portfolio_indicator.pnl_monthly.value_pct,
                    portfolio_indicator.pnl_yearly.value_pct,
                    portfolio_indicator.pnl_for_3_months.value_pct,
                    portfolio_indicator.pnl_for_1_year.value_pct,
                    portfolio_indicator.pnl_current.value,
                    portfolio_indicator.pnl_daily.value,
                    portfolio_indicator.pnl_weekly.value,
                    portfolio_indicator.pnl_monthly.value,
                    portfolio_indicator.pnl_yearly.value,
                    portfolio_indicator.pnl_for_3_months.value,
                    portfolio_indicator.pnl_for_1_year.value,
                    portfolio_indicator.volatility_3_month,
                    portfolio_indicator.volatility_1_year,
                    portfolio_indicator.earning,
                    portfolio_indicator.earning_latent
                )
                .as_bytes(),
            )?;
        }

        if !have_line {
            std::fs::remove_file(filename)?;
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
          "Date;Instrument;Spot(Close);Quantity;Unit Price;Valuation;Nominal;Dividends;Tax;P&L(%);P&L Daily(%);P&L Weekly(%);P&L Monthly(%);P&L Yearly(%);P&L for 3 Months(%);P&L for one Year(%);P&L;P&L Daily;P&L Weekly;P&L Monthly;P&L Yearly;P&L for 3 Months;P&L for one Year;Volatility 3M;Volatility 1Y;Earning;Earning + Valuation\n".as_bytes(),
        )?;
        let mut have_line = false;
        for position_indicator in self.indicators.by_instrument_name(instrument_name) {
            if self
                .filter_indicators
                .map_or(false, |date| date > position_indicator.date)
            {
                continue;
            }
            have_line = true;
            output_stream.write_all(
                format!(
                    "{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}\n",
                    position_indicator.date.format("%Y-%m-%d"),
                    instrument_name,
                    position_indicator.spot.close(),
                    position_indicator.quantity,
                    position_indicator.unit_price,
                    position_indicator.valuation,
                    position_indicator.nominal,
                    position_indicator.dividends,
                    position_indicator.tax,
                    position_indicator.pnl_current.value_pct,
                    position_indicator.pnl_daily.value_pct,
                    position_indicator.pnl_weekly.value_pct,
                    position_indicator.pnl_monthly.value_pct,
                    position_indicator.pnl_yearly.value_pct,
                    position_indicator.pnl_for_3_months.value_pct,
                    position_indicator.pnl_for_1_year.value_pct,
                    position_indicator.pnl_current.value,
                    position_indicator.pnl_daily.value,
                    position_indicator.pnl_weekly.value,
                    position_indicator.pnl_monthly.value,
                    position_indicator.pnl_yearly.value,
                    position_indicator.pnl_for_3_months.value,
                    position_indicator.pnl_for_1_year.value,
                    position_indicator.volatility_3_month,
                    position_indicator.volatility_1_year,
                    position_indicator.earning,
                    position_indicator.earning_latent,
                )
                .as_bytes(),
            )?;
        }

        if !have_line {
            std::fs::remove_file(filename)?;
        }

        Ok(())
    }
}

impl<'a> Output for CsvOutput<'a> {
    fn write_indicators(&mut self) -> Result<(), Error> {
        let filename = format!("{}/indicators_{}.csv", self.output_dir, self.portfolio.name);
        self.write_position_indicators(&filename)?;

        for instrument_name in self.portfolio.get_instrument_name_list() {
            let filename = format!(
                "{}/indicators_{}_{}.csv",
                self.output_dir, self.portfolio.name, instrument_name
            );
            self.write_position_instrument_indicators(instrument_name, &filename)?;
        }

        let filename = format!("{}/heat_map_{}.csv", self.output_dir, self.portfolio.name);
        self.write_heat_map(&filename)?;

        for instrument_name in self.portfolio.get_instrument_name_list() {
            let filename = format!(
                "{}/heat_map_{}_{}.csv",
                self.output_dir, self.portfolio.name, instrument_name
            );
            self.write_instrument_heat_map(instrument_name, &filename)?;
        }

        let filename = format!(
            "{}/distribution_by_region_{}.csv",
            self.output_dir, self.portfolio.name
        );
        self.write_distribution_by_region(&filename)?;

        let filename = format!(
            "{}/distribution_global_{}.csv",
            self.output_dir, self.portfolio.name
        );
        self.write_distribution_global_by_instrument(&filename)?;

        for region_name in self.portfolio.get_region_name_list() {
            let filename = format!(
                "{}/distribution_{}_{}.csv",
                self.output_dir, self.portfolio.name, region_name
            );
            self.write_distribution_by_instrument(region_name, &filename)?;
        }

        Ok(())
    }
}
