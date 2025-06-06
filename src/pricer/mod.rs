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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::historical::DataFrame;
    use crate::marketdata::{Currency, Instrument, Market};
    use crate::portfolio::{Position, Trade, Way};
    use std::rc::Rc;

    #[derive(Default)]
    struct MockSpotProvider {
        pub instrument_feched: Vec<(String, Date, Date)>,
    }

    impl Provider for MockSpotProvider {
        fn fetch(&mut self, instrument: &Instrument, begin: Date, end: Date) -> Result<(), Error> {
            self.instrument_feched
                .push((instrument.name.clone(), begin, end));
            Ok(())
        }
        fn latest(&self, _instrument: &Instrument, _date: Date) -> Option<&DataFrame> {
            None
        }
    }

    fn make_date_(year: i32, month: u32, day: u32) -> Date {
        chrono::NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn make_instrument_(name: &str) -> Rc<Instrument> {
        let currency = Rc::new(Currency {
            name: String::from("EUR"),
            parent_currency: None,
        });

        let market = Rc::new(Market {
            name: String::from("EPA"),
            description: String::from("EPA"),
        });

        Rc::new(Instrument {
            name: String::from(name),
            isin: String::from("ISIN"),
            description: String::from("description"),
            market,
            currency,
            ticker_yahoo: None,
            region: None,
            fund_category: String::from("category"),
            dividends: None,
        })
    }

    fn build_portfolio_empty_() -> Portfolio {
        let currency = Rc::new(Currency {
            name: String::from("EUR"),
            parent_currency: None,
        });

        Portfolio {
            name: "PTF".to_string(),
            currency,
            positions: Vec::new(),
            cash: Vec::new(),
        }
    }

    fn build_portfolio_fetch_() -> Portfolio {
        let currency = Rc::new(Currency {
            name: String::from("EUR"),
            parent_currency: None,
        });

        let instrument1 = make_instrument_("ESE");
        let instrument2 = make_instrument_("PAEEM");

        Portfolio {
            name: "PTF".to_string(),
            currency,
            positions: vec![
                Position {
                    instrument: instrument1,
                    trades: vec![
                        Trade {
                            date: chrono::DateTime::parse_from_rfc3339("2025-01-01T10:00:00-00:00")
                                .unwrap()
                                .naive_local(),
                            way: Way::Buy,
                            quantity: 14.0,
                            price: 21.5,
                            fees: 1.55,
                        },
                        Trade {
                            date: chrono::DateTime::parse_from_rfc3339("2025-02-01T10:00:00-00:00")
                                .unwrap()
                                .naive_local(),
                            way: Way::Buy,
                            quantity: 20.0,
                            price: 21.5,
                            fees: 1.55,
                        },
                        Trade {
                            date: chrono::DateTime::parse_from_rfc3339("2025-03-01T10:00:00-00:00")
                                .unwrap()
                                .naive_local(),
                            way: Way::Buy,
                            quantity: 14.0,
                            price: 20.5,
                            fees: 1.55,
                        },
                        Trade {
                            date: chrono::DateTime::parse_from_rfc3339("2025-04-01T10:00:00-00:00")
                                .unwrap()
                                .naive_local(),
                            way: Way::Buy,
                            quantity: 22.0,
                            price: 21.5,
                            fees: 1.55,
                        },
                    ],
                },
                Position {
                    instrument: instrument2,
                    trades: vec![
                        Trade {
                            date: chrono::DateTime::parse_from_rfc3339("2025-02-01T10:00:00-00:00")
                                .unwrap()
                                .naive_local(),
                            way: Way::Buy,
                            quantity: 20.0,
                            price: 21.5,
                            fees: 1.55,
                        },
                        Trade {
                            date: chrono::DateTime::parse_from_rfc3339("2025-03-01T10:00:00-00:00")
                                .unwrap()
                                .naive_local(),
                            way: Way::Sell,
                            quantity: 20.0,
                            price: 20.5,
                            fees: 1.55,
                        },
                    ],
                },
            ],
            cash: Vec::new(),
        }
    }

    #[test]
    fn portfolio_indicators_empty() {
        let portfolio = build_portfolio_empty_();
        let mut spot_provider = MockSpotProvider::default();
        let begin = make_date_(2025, 1, 1);
        let end = make_date_(2025, 3, 4);
        let result =
            PortfolioIndicators::from_portfolio(&portfolio, begin, end, &mut spot_provider);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.begin == begin);
        assert!(result.end == end);
        assert!(result.portfolios.is_empty());
    }

    #[test]
    fn portfolio_indicators_fetch_() {
        let portfolio = build_portfolio_fetch_();
        let mut spot_provider = MockSpotProvider::default();
        let begin = make_date_(2025, 1, 1);
        let end = make_date_(2025, 3, 4);
        let result =
            PortfolioIndicators::from_portfolio(&portfolio, begin, end, &mut spot_provider);
        assert!(result.is_ok());
        assert!(spot_provider.instrument_feched.len() == 2);

        let data = spot_provider
            .instrument_feched
            .iter()
            .find(|(name, _, _)| name == "ESE");
        assert!(data.is_some());
        let data = data.unwrap();
        assert!(data.1 == begin);
        assert!(data.2 == end);

        let data = spot_provider
            .instrument_feched
            .iter()
            .find(|(name, _, _)| name == "PAEEM");
        assert!(data.is_some());
        let data = data.unwrap();
        assert!(data.1 == make_date_(2025, 2, 1));
        assert!(data.2 == make_date_(2025, 3, 1));
    }
}
