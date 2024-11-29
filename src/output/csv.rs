use chrono::Datelike;

use super::Output;
use crate::alias::Date;
use crate::error::Error;
use crate::portfolio::Portfolio;
use crate::pricer::{
    HeatMap, HeatMapPeriod, InstrumentIndicator, PortfolioIndicators, PositionIndicators,
    RegionIndicator, RegionIndicatorInstrument,
};

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;

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

    fn write_distribution_by_region(
        &self,
        filename: &str,
        indicators: &Vec<RegionIndicator>,
    ) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        for indicator in indicators {
            output_stream.write_all(
                format!(
                    "{};{}\n",
                    indicator.region_name, indicator.valuation_percent
                )
                .as_bytes(),
            )?;
        }
        Ok(())
    }

    fn write_distribution_by_instrument(
        &self,
        filename: &str,
        indicators: &Vec<RegionIndicatorInstrument>,
    ) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        for indicator in indicators {
            output_stream.write_all(
                format!(
                    "{};{}\n",
                    indicator.instrument.name, indicator.valuation_percent
                )
                .as_bytes(),
            )?;
        }
        Ok(())
    }

    fn write_distribution_global_by_instrument(
        &self,
        filename: &str,
        indicators: &Vec<InstrumentIndicator>,
    ) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        for indicator in indicators {
            output_stream.write_all(
                format!(
                    "{};{}\n",
                    indicator.instrument.name, indicator.valuation_percent
                )
                .as_bytes(),
            )?;
        }
        Ok(())
    }

    fn write_heat_map_monthly(&self, filename: &str, heat_map: HeatMap) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        output_stream
            .write_all("Year;Jan;Feb;Mar;Apr;May;Jun;Jul;Aug,Sep;Oct;Nov;Dec\n".as_bytes())?;

        let mut data: BTreeMap<i32, [Option<f64>; 12]> = Default::default();
        for (date, value) in heat_map.data {
            let row = data.entry(date.year()).or_default();
            row[date.month0() as usize] = Some(100.0 * value);
        }

        for (year, values) in data {
            let mut line = format!("{}", year);
            for value in values {
                if let Some(pct) = value {
                    line += &format!("{}", pct);
                }
                line += ";";
            }
            line += "\n";
            output_stream.write_all(line.as_bytes())?;
        }

        Ok(())
    }

    fn write_heat_map_yearly(&self, filename: &str, heat_map: HeatMap) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        output_stream.write_all("Year;Value\n".as_bytes())?;

        for (date, value) in heat_map.data {
            output_stream.write_all(format!("{};{}\n", date.year(), 100.0 * value).as_bytes())?;
        }

        Ok(())
    }

    fn write_position_indicators(&self, filename: &str) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        output_stream.write_all(
            "Date;Valuation;Nominal;Incoming Transfert;Outcoming Transfert;Cash;Dividends;Fees;P&L;P&L(%);TWR;Earning;Earning Latent\n".as_bytes(),
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
                    "{};{};{};{};{};{};{};{};{};{};{};{};{}\n",
                    portfolio_indicator.date.format("%Y-%m-%d"),
                    portfolio_indicator.valuation,
                    portfolio_indicator.nominal,
                    portfolio_indicator.incoming_transfer,
                    portfolio_indicator.outcoming_transfer,
                    portfolio_indicator.cash,
                    portfolio_indicator.dividends,
                    portfolio_indicator.fees,
                    portfolio_indicator.pnl_currency,
                    portfolio_indicator.pnl_percent,
                    portfolio_indicator.twr,
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
        indicators: PositionIndicators,
        filename: &str,
    ) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        output_stream.write_all(
          "Date;Instrument;Spot(Close);Quantity;Quantity Buy;Quantity Sell;Unit Price;Valuation;Nominal;Cashflow;Dividends;Fees;P&L;P&L(%);TWR;Earning;Earning Latent;Is Close\n".as_bytes(),
        )?;
        let mut have_line = false;
        for position_indicator in indicators
            .positions
            .into_iter()
            .filter(|item| self.filter_indicators.map_or(true, |date| date < item.date))
        {
            have_line = true;
            output_stream.write_all(
                format!(
                    "{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}\n",
                    position_indicator.date.format("%Y-%m-%d"),
                    position_indicator.instrument.name,
                    position_indicator.spot.close,
                    position_indicator.quantity,
                    position_indicator.quantity_buy,
                    position_indicator.quantity_sell,
                    position_indicator.unit_price,
                    position_indicator.valuation,
                    position_indicator.nominal,
                    position_indicator.cashflow,
                    position_indicator.dividends,
                    position_indicator.fees,
                    position_indicator.pnl_currency,
                    position_indicator.pnl_percent,
                    position_indicator.twr,
                    position_indicator.earning,
                    position_indicator.earning_latent,
                    position_indicator.is_close,
                )
                .as_bytes(),
            )?;
            if position_indicator.is_close {
                break;
            }
        }

        if !have_line {
            std::fs::remove_file(filename)?;
        }

        Ok(())
    }
}

impl Output for CsvOutput<'_> {
    fn write(&mut self) -> Result<(), Error> {
        let filename = format!("{}/indicators_{}.csv", self.output_dir, self.portfolio.name);
        self.write_position_indicators(&filename)?;

        for instrument_name in self.portfolio.get_instrument_name_list() {
            for position_index in self.indicators.get_position_index_list(instrument_name) {
                let position_indicators = self
                    .indicators
                    .get_position_indicators(instrument_name, position_index);

                let filename = format!(
                    "{}/heat_map_{}_{}_{}.csv",
                    self.output_dir, self.portfolio.name, instrument_name, position_index
                );
                let heat_map = HeatMap::from_positions(
                    &position_indicators,
                    HeatMapPeriod::Monthly,
                    |indicator| indicator.pnl_percent,
                );
                self.write_heat_map_monthly(&filename, heat_map)?;

                let filename = format!(
                    "{}/heat_map_yearly_{}_{}_{}.csv",
                    self.output_dir, self.portfolio.name, instrument_name, position_index
                );
                let heat_map = HeatMap::from_positions(
                    &position_indicators,
                    HeatMapPeriod::Yearly,
                    |indicator| indicator.pnl_percent,
                );
                self.write_heat_map_yearly(&filename, heat_map)?;

                let position_filename = format!(
                    "{}/indicators_{}_{}_{}.csv",
                    self.output_dir, self.portfolio.name, instrument_name, position_index
                );
                self.write_position_instrument_indicators(position_indicators, &position_filename)?;
            }
        }

        if let Some(indicator) = self.indicators.portfolios.last() {
            let region_indicators = RegionIndicator::from_portfolio(indicator);
            let filename = format!(
                "{}/distribution_by_region_{}.csv",
                self.output_dir, self.portfolio.name
            );
            self.write_distribution_by_region(&filename, &region_indicators)?;
            for region_indicator in region_indicators {
                let filename = format!(
                    "{}/distribution_{}_{}.csv",
                    self.output_dir, self.portfolio.name, region_indicator.region_name
                );
                self.write_distribution_by_instrument(&filename, &region_indicator.instruments)?;
            }

            let instrument_indicators = InstrumentIndicator::from_portfolio(indicator);
            let filename = format!(
                "{}/distribution_global_{}.csv",
                self.output_dir, self.portfolio.name
            );
            self.write_distribution_global_by_instrument(&filename, &instrument_indicators)?;
        }

        let filename = format!("{}/heat_map_{}.csv", self.output_dir, self.portfolio.name);
        let heat_map =
            HeatMap::from_portfolios(self.indicators, HeatMapPeriod::Monthly, |indicator| {
                indicator.pnl_percent
            });
        self.write_heat_map_monthly(&filename, heat_map)?;

        let filename = format!(
            "{}/heat_map_yearly_{}.csv",
            self.output_dir, self.portfolio.name
        );
        let heat_map =
            HeatMap::from_portfolios(self.indicators, HeatMapPeriod::Yearly, |indicator| {
                indicator.pnl_percent
            });
        self.write_heat_map_yearly(&filename, heat_map)?;

        Ok(())
    }
}
