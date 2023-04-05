use crate::alias::Date;
use crate::error::{Error, ErrorKind};
use crate::historical::{DataFrame, Provider};
use crate::marketdata::Instrument;
use crate::portfolio::{Portfolio, Position};
use std::rc::Rc;

use log::info;

mod iterator;
use iterator::DateByStepIterator;

#[allow(dead_code)]
pub enum Step {
    Year,
    Month,
    Day,
    Week,
}

pub struct PositionIndicator {
    spot: DataFrame,
    instrument: Rc<Instrument>,
}

impl PositionIndicator {
    pub fn from_position<P>(
        position: &Position,
        _begin_period: Date,
        end_period: Date,
        spot_provider: &mut P,
    ) -> Result<PositionIndicator, Error>
    where
        P: Provider,
    {
        let spot = spot_provider
            .latest(&position.instrument, end_period)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::Historical,
                    format!(
                        "not spot for '{}' latest {}",
                        position.instrument.name,
                        end_period.format("%Y-%m-%d")
                    ),
                )
            })?;

        Ok(PositionIndicator {
            spot: *spot,
            instrument: position.instrument.clone(),
        })
    }
}

pub struct PortfolioIndicator {
    date: Date,
    positions: Vec<PositionIndicator>,
}

impl PortfolioIndicator {
    pub fn from_portfolio<P>(
        portfolio: &Portfolio,
        begin_period: Date,
        end_period: Date,
        spot_provider: &mut P,
    ) -> Result<PortfolioIndicator, Error>
    where
        P: Provider,
    {
        let positions = PortfolioIndicator::make_positions_(
            portfolio,
            begin_period,
            end_period,
            spot_provider,
        )?;

        Ok(PortfolioIndicator {
            date: end_period,
            positions,
        })
    }

    fn make_positions_<P>(
        portfolio: &Portfolio,
        begin_period: Date,
        end_period: Date,
        spot_provider: &mut P,
    ) -> Result<Vec<PositionIndicator>, Error>
    where
        P: Provider,
    {
        let result = portfolio
            .positions
            .iter()
            .fold((Vec::new(), None), |accu, value| {
                if accu.1.is_some() {
                    accu
                } else {
                    let mut data = accu.0;
                    match PositionIndicator::from_position(
                        value,
                        begin_period,
                        end_period,
                        spot_provider,
                    ) {
                        Ok(value) => {
                            data.push(value);
                            (data, None)
                        }
                        Err(error) => (data, Some(error)),
                    }
                }
            });
        if let Some(error) = result.1 {
            Err(error)
        } else {
            Ok(result.0)
        }
    }
}

pub struct PortfolioIndicators {
    portfolios: Vec<PortfolioIndicator>,
}

impl PortfolioIndicators {
    pub fn from_portfolio<P>(
        portfolio: &Portfolio,
        begin: Date,
        end: Date,
        step: Step,
        spot_provider: &mut P,
    ) -> Result<PortfolioIndicators, Error>
    where
        P: Provider,
    {
        info!("request all market data historical");
        for position in portfolio.positions.iter() {
            if let Some(trade) = position.trades.first() {
                let instrument_begin = trade.date.date();
                if instrument_begin < end {
                    spot_provider.fetch(&position.instrument, instrument_begin, end)?;
                }
            }
        }
        info!("request all market data historical done");

        let portfolios =
            PortfolioIndicators::make_portfolios_(portfolio, begin, end, step, spot_provider)?;

        Ok(PortfolioIndicators { portfolios })
    }

    fn make_portfolios_<P>(
        portfolio: &Portfolio,
        begin: Date,
        end: Date,
        step: Step,
        spot_provider: &mut P,
    ) -> Result<Vec<PortfolioIndicator>, Error>
    where
        P: Provider,
    {
        let result = DateByStepIterator::new(begin, end, step).fold(
            (begin, Vec::new(), None),
            |accu, end_period| {
                if accu.2.is_some() {
                    accu
                } else {
                    let mut data = accu.1;
                    let begin_period = accu.0;
                    match PortfolioIndicator::from_portfolio(
                        portfolio,
                        begin_period,
                        end_period,
                        spot_provider,
                    ) {
                        Ok(value) => {
                            data.push(value);
                            (end_period, data, None)
                        }
                        Err(error) => (end_period, data, Some(error)),
                    }
                }
            },
        );
        if let Some(error) = result.2 {
            Err(error)
        } else {
            Ok(result.1)
        }
    }
}
