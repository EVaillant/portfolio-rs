use crate::error::Error;
use crate::historical::Provider;
use crate::portfolio::Portfolio;
use crate::{alias::Date, marketdata::Instrument};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use log::{error, info};

mod portfolio;
mod position;
mod primitive;

pub use portfolio::PortfolioIndicator;
pub use position::PositionIndicator;

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

pub struct RegionIndicatorInstrument {
    pub instrument: Rc<Instrument>,
    pub valuation_percent: f64,
}

pub struct RegionIndicator {
    pub region_name: String,
    pub valuation_percent: f64,
    pub instruments: Vec<RegionIndicatorInstrument>,
}

impl RegionIndicator {
    pub fn from_portfolio(indicator: &PortfolioIndicator) -> Vec<Self> {
        let regions = indicator
            .positions
            .iter()
            .filter(|position| !position.is_close)
            .map(|position| &position.instrument.region)
            .collect::<HashSet<_>>();

        let valuation = indicator
            .positions
            .iter()
            .filter(|position| !position.is_close)
            .map(|position| &position.valuation)
            .sum::<f64>();

        regions
            .into_iter()
            .map(|region| {
                let mut valuation_by_instrument: HashMap<Rc<Instrument>, f64> = Default::default();
                let mut valuation_by_region = 0.0;
                indicator
                    .positions
                    .iter()
                    .filter(|position| !position.is_close && position.instrument.region == *region)
                    .for_each(|position| {
                        let value = valuation_by_instrument
                            .entry(position.instrument.clone())
                            .or_insert(0.0);
                        *value += position.valuation;
                        valuation_by_region += position.valuation;
                    });
                RegionIndicator {
                    region_name: region.to_string(),
                    valuation_percent: valuation_by_region / valuation,
                    instruments: valuation_by_instrument
                        .iter()
                        .map(|(key, value)| RegionIndicatorInstrument {
                            instrument: key.clone(),
                            valuation_percent: value / valuation_by_region,
                        })
                        .collect(),
                }
            })
            .collect()
    }
}

pub struct InstrumentIndicator {
    pub instrument: Rc<Instrument>,
    pub valuation_percent: f64,
}

impl InstrumentIndicator {
    pub fn from_portfolio(indicator: &PortfolioIndicator) -> Vec<Self> {
        let instruments = indicator
            .positions
            .iter()
            .filter(|position| !position.is_close)
            .map(|position| position.instrument.clone())
            .collect::<HashSet<_>>();

        let valuation = indicator
            .positions
            .iter()
            .filter(|position| !position.is_close)
            .map(|position| &position.valuation)
            .sum::<f64>();

        instruments
            .into_iter()
            .map(|instrument| {
                let valuation_by_instrument = indicator
                    .positions
                    .iter()
                    .filter(|position| !position.is_close && position.instrument == instrument)
                    .map(|position| &position.valuation)
                    .sum::<f64>();
                InstrumentIndicator {
                    instrument: instrument.clone(),
                    valuation_percent: valuation_by_instrument / valuation,
                }
            })
            .collect()
    }
}
