use crate::alias::Date;
use crate::error::Error;
use crate::historical::{DataFrame, Provider};
use crate::marketdata::Instrument;
use crate::portfolio::{Portfolio, Position, Way};
use std::fs::File;
use std::io::Write;
use std::rc::Rc;

use log::info;

mod iterator;
use iterator::DateByStepIterator;

#[derive(Copy, Clone)]
pub enum Step {
    Year,
    Month,
    Day,
    Week,
}

impl Step {
    pub fn to_string(&self) -> &str {
        match self {
            Step::Day => "daily",
            Step::Month => "monthly",
            Step::Week => "weekly",
            Step::Year => "yearly",
        }
    }
}

pub struct PositionIndicator {
    pub spot: DataFrame,
    pub instrument: Rc<Instrument>,
    pub quantity: f64,
    pub quantity_buy: f64,
    pub quantity_sell: f64,
    pub unit_price: f64,
    pub valuation: f64,
    pub dividends: f64,
    pub tax: f64,
    pub latent: f64,
    pub latent_in_percent: f64,
    pub earning: f64,
    pub pnl: f64,
    pub pnl_in_percent: f64,
}

impl PositionIndicator {
    pub fn from_position<P>(
        position: &Position,
        begin_period: Date,
        end_period: Date,
        spot_provider: &mut P,
    ) -> Option<PositionIndicator>
    where
        P: Provider,
    {
        if let Some(spot) = spot_provider.latest(&position.instrument, end_period) {
            if spot.date() == &end_period || spot.date() > &begin_period {
                let (quantity, quantity_buy, quantity_sell, unit_price, tax) =
                    PositionIndicator::compute_quantity_(position, end_period);

                let valuation = unit_price * quantity;
                let dividends = position
                    .instrument
                    .dividends
                    .as_ref()
                    .map(|dividends| {
                        dividends
                            .iter()
                            .map(|dividend| {
                                let quantity = PositionIndicator::compute_quantity_(
                                    position,
                                    dividend.record_date.date(),
                                )
                                .0;
                                dividend.value * quantity
                            })
                            .sum()
                    })
                    .unwrap_or_else(|| 0.0);

                let latent = (spot.close() - unit_price) * quantity;
                let latent_in_percent = latent / valuation;
                let earning = position
                    .trades
                    .iter()
                    .filter(|trade| trade.way == Way::Sell)
                    .fold(dividends, |dividends, trade| {
                        dividends + trade.price * trade.quantity + trade.tax
                    });
                let pnl = earning + latent;
                let pnl_in_percent = pnl / valuation;

                Some(PositionIndicator {
                    spot: *spot,
                    instrument: position.instrument.clone(),
                    quantity,
                    quantity_buy,
                    quantity_sell,
                    unit_price,
                    valuation,
                    dividends,
                    tax,
                    latent,
                    latent_in_percent,
                    earning,
                    pnl,
                    pnl_in_percent,
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    fn compute_quantity_(position: &Position, date: Date) -> (f64, f64, f64, f64, f64) {
        position
            .trades
            .iter()
            .filter(|trade| trade.date.date() <= date)
            .fold(
                (0.0, 0.0, 0.0, 0.0, 0.0),
                |(mut quantity, mut quantity_buy, mut quantity_sell, mut unit_price, mut tax),
                 trade| {
                    match trade.way {
                        Way::Sell => {
                            quantity -= trade.quantity;
                            quantity_sell += trade.quantity;
                        }
                        Way::Buy => {
                            unit_price =
                                (quantity * unit_price + trade.price * trade.quantity + trade.tax)
                                    / (quantity + trade.quantity);
                            quantity += trade.quantity;
                            quantity_buy += trade.quantity;
                        }
                    };
                    tax += trade.tax;
                    (quantity, quantity_buy, quantity_sell, unit_price, tax)
                },
            )
    }
}

pub struct PortfolioIndicator {
    pub date: Date,
    pub positions: Vec<PositionIndicator>,
    pub valuation: f64,
    pub dividends: f64,
    pub tax: f64,
    pub latent: f64,
    pub latent_in_percent: f64,
    pub earning: f64,
    pub pnl: f64,
    pub pnl_in_percent: f64,
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
        let positions =
            PortfolioIndicator::make_positions_(portfolio, begin_period, end_period, spot_provider);

        let (valuation, dividends, tax, latent, earning, pnl) = positions.iter().fold(
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            |(valuation, dividends, tax, latent, earning, pnl), position_indicator| {
                (
                    valuation + position_indicator.valuation,
                    dividends + position_indicator.dividends,
                    tax + position_indicator.tax,
                    latent + position_indicator.latent,
                    earning + position_indicator.earning,
                    pnl + position_indicator.pnl,
                )
            },
        );
        let latent_in_percent = latent / valuation;
        let pnl_in_percent = pnl / valuation;

        Ok(PortfolioIndicator {
            date: end_period,
            positions,
            valuation,
            dividends,
            tax,
            latent,
            latent_in_percent,
            earning,
            pnl,
            pnl_in_percent,
        })
    }

    fn make_positions_<P>(
        portfolio: &Portfolio,
        begin_period: Date,
        end_period: Date,
        spot_provider: &mut P,
    ) -> Vec<PositionIndicator>
    where
        P: Provider,
    {
        let mut data = Vec::new();
        portfolio.positions.iter().for_each(|position| {
            if let Some(value) =
                PositionIndicator::from_position(position, begin_period, end_period, spot_provider)
            {
                data.push(value);
            }
        });
        data
    }
}

pub struct PortfolioIndicators {
    pub portfolios: Vec<PortfolioIndicator>,
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
        info!(
            "request all market data historical for {} from {} to {} for {} pricing",
            portfolio.name,
            begin.format("%Y-%m-%d"),
            end.format("%Y-%m-%d"),
            step.to_string(),
        );

        for position in portfolio.positions.iter() {
            if let Some(trade) = position.trades.first() {
                let instrument_begin = trade.date.date();
                if instrument_begin < end {
                    spot_provider.fetch(&position.instrument, instrument_begin, end)?;
                }
            }
        }
        info!("request all market data historical done");

        info!("start to price portfolios");
        let portfolios =
            PortfolioIndicators::make_portfolios_(portfolio, begin, end, step, spot_provider)?;
        info!("price portfolios is finished");

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
        let mut data = Vec::new();
        let result =
            DateByStepIterator::new(begin, end, step).fold((begin, None), |accu, end_period| {
                if accu.1.is_some() {
                    accu
                } else {
                    let begin_period = accu.0;
                    match PortfolioIndicator::from_portfolio(
                        portfolio,
                        begin_period,
                        end_period,
                        spot_provider,
                    ) {
                        Ok(value) => {
                            if !value.positions.is_empty() {
                                data.push(value);
                            }
                            (end_period, None)
                        }
                        Err(error) => (end_period, Some(error)),
                    }
                }
            });
        if let Some(error) = result.1 {
            Err(error)
        } else {
            Ok(data)
        }
    }

    pub fn dump_indicators_in_csv(&self, filename: &str) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        output_stream.write_all(
            "Date;Valuation;Dividends;Tax;Latent;Latent(%);Earning;Pnl;Pnl(%)\n".as_bytes(),
        )?;
        self.portfolios.iter().for_each(|portfolio_indicator| {
            output_stream
                .write_all(
                    format!(
                        "{};{};{};{};{};{};{};{};{}\n",
                        portfolio_indicator.date.format("%Y-%m-%d"),
                        portfolio_indicator.valuation,
                        portfolio_indicator.dividends,
                        portfolio_indicator.tax,
                        portfolio_indicator.latent,
                        portfolio_indicator.latent_in_percent,
                        portfolio_indicator.earning,
                        portfolio_indicator.pnl,
                        portfolio_indicator.pnl_in_percent
                    )
                    .as_bytes(),
                )
                .unwrap();
        });
        Ok(())
    }

    pub fn dump_instrument_indicators_in_csv(
        &self,
        instrument_name: &str,
        filename: &str,
    ) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        output_stream.write_all(
            "Date;Instrument;Spot(Close);Quantity;Unit Price;Valuation;Dividends;Tax;Latent;Latent(%);Earning;Pnl;Pnl(%)\n".as_bytes(),
        )?;

        self.portfolios
            .iter()
            .flat_map(|item| {
                item.positions
                    .iter()
                    .find(|item| item.instrument.name == instrument_name)
            })
            .for_each(|position_indicator| {
                output_stream
                    .write_all(
                        format!(
                            "{};{};{};{};{};{};{};{};{};{};{};{};{}\n",
                            position_indicator.spot.date().format("%Y-%m-%d"),
                            instrument_name,
                            position_indicator.spot.close(),
                            position_indicator.quantity,
                            position_indicator.unit_price,
                            position_indicator.valuation,
                            position_indicator.dividends,
                            position_indicator.tax,
                            position_indicator.latent,
                            position_indicator.latent_in_percent,
                            position_indicator.earning,
                            position_indicator.pnl,
                            position_indicator.pnl_in_percent
                        )
                        .as_bytes(),
                    )
                    .unwrap();
            });
        Ok(())
    }
}
