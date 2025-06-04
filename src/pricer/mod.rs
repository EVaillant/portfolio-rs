use crate::alias::Date;
use crate::error::Error;
use crate::historical::Provider;
use crate::portfolio::Portfolio;
use std::collections::{HashMap, HashSet};

use log::{error, info};

mod heat_map;
mod instrument;
mod portfolio;
mod position;
mod primitive;
mod region;

pub use heat_map::{HeatMap, HeatMapPeriod};
pub use instrument::InstrumentIndicator;
pub use portfolio::PortfolioIndicator;
pub use position::{ClosePositionIndicator, PositionIndicator};
pub use region::{RegionIndicator, RegionIndicatorInstrument};

pub struct PositionIndicators<'a> {
    pub begin: Date,
    pub end: Date,
    pub instrument_name: String,
    pub position_index: usize,
    pub positions: Vec<&'a PositionIndicator>,
}

pub struct PortfolioIndicators {
    pub begin: Date,
    pub end: Date,
    pub portfolios: Vec<PortfolioIndicator>,
}

impl PortfolioIndicators {
    pub fn from_portfolio<P>(
        portfolio: &Portfolio,
        begin: Date,
        end: Date,
        spot_provider: &mut P,
    ) -> Result<PortfolioIndicators, Error>
    where
        P: Provider,
    {
        info!(
            "request all market data historical for {} from {} to {} pricing",
            portfolio.name,
            begin.format("%Y-%m-%d"),
            end.format("%Y-%m-%d"),
        );

        for position in portfolio.positions.iter() {
            if let Some(trade) = position.trades.first() {
                let instrument_begin = trade.date.date();
                if instrument_begin <= end {
                    let instrument_end = position
                        .get_close_date()
                        .map(|date_time| date_time.date())
                        .unwrap_or(end);
                    spot_provider.fetch(&position.instrument, instrument_begin, instrument_end)?;
                }
            }
        }
        info!("request all market data historical done");

        info!("start to price portfolios");
        let portfolios =
            PortfolioIndicators::make_portfolios_(portfolio, begin, end, spot_provider);
        info!("price portfolios is finished");

        Ok(PortfolioIndicators {
            begin,
            end,
            portfolios,
        })
    }

    pub fn get_position_index_list(&self, name: &str) -> HashSet<usize> {
        let mut result = HashSet::new();
        if let Some(indicator) = self.portfolios.last() {
            result = indicator
                .positions
                .iter()
                .filter(|item| item.instrument.name == name)
                .map(|item| item.position_index)
                .collect();
        }
        result
    }

    pub fn get_position_indicators<'a>(
        &'a self,
        instrument_name: &str,
        position_index: usize,
    ) -> PositionIndicators<'a> {
        let positions = self
            .portfolios
            .iter()
            .flat_map(|portfolio| {
                portfolio.positions.iter().filter(|item| {
                    item.instrument.name == instrument_name && item.position_index == position_index
                })
            })
            .collect();

        PositionIndicators {
            begin: self.begin,
            end: self.end,
            instrument_name: instrument_name.to_string(),
            position_index,
            positions,
        }
    }

    fn make_positions_date_<P>(
        portfolio: &Portfolio,
        begin: Date,
        end: Date,
        spot_provider: &mut P,
    ) -> HashMap<Date, Vec<PositionIndicator>>
    where
        P: Provider,
    {
        let mut result: HashMap<Date, Vec<PositionIndicator>> = Default::default();
        for (position_index, position) in portfolio.positions.iter().enumerate() {
            let mut indicators = Vec::new();
            if let Some(trade) = position.trades.first() {
                let begin = std::cmp::max(trade.date.date(), begin);
                for date in begin.iter_days().take_while(|item| item <= &end) {
                    if let Some(spot) = spot_provider.latest(&position.instrument, date) {
                        let indicator = PositionIndicator::from_position(
                            position,
                            date,
                            position_index,
                            spot,
                            &indicators,
                        );
                        indicators.push(indicator);
                    } else {
                        error!(
                            "no spot on {}/{} at {} and before skip position pricing",
                            position.instrument.name, position_index, date
                        );
                    }
                }
            }
            for indicator in indicators {
                result.entry(indicator.date).or_default().push(indicator);
            }
        }
        result
    }

    fn make_portfolios_<P>(
        portfolio: &Portfolio,
        begin: Date,
        end: Date,
        spot_provider: &mut P,
    ) -> Vec<PortfolioIndicator>
    where
        P: Provider,
    {
        let mut indicators = Vec::new();
        let mut positions_by_date =
            PortfolioIndicators::make_positions_date_(portfolio, begin, end, spot_provider);
        for date in begin.iter_days().take_while(|item| item <= &end) {
            if let Some(position_indicators) = positions_by_date.remove(&date) {
                if position_indicators.is_empty() {
                    continue;
                }

                let indicator = PortfolioIndicator::from_portfolio(
                    portfolio,
                    date,
                    position_indicators,
                    &indicators,
                );

                indicators.push(indicator);
            }
        }

        indicators
    }
}
