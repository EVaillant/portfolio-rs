use crate::alias::Date;
use crate::error::Error;
use crate::historical::{DataFrame, Provider};
use crate::marketdata::Instrument;
use crate::portfolio::{CashVariationSource, Portfolio, Position, Way};
use std::fs::File;
use std::io::Write;
use std::rc::Rc;

use log::{debug, error, info};

mod pnl;
use pnl::{make_pnls, Pnl};

pub struct PositionIndicator {
    pub spot: DataFrame,
    pub instrument: Rc<Instrument>,
    pub quantity: f64,
    pub quantity_buy: f64,
    pub quantity_sell: f64,
    pub unit_price: f64,
    pub valuation: f64,
    pub nominal: f64,
    pub dividends: f64,
    pub tax: f64,
    pub current_pnl: Pnl,
    pub daily_pnl: Pnl,
    pub weekly_pnl: Pnl,
    pub monthly_pnl: Pnl,
    pub yearly_pnl: Pnl,
    pub earning: f64,
    pub earning_latent: f64,
}

impl PositionIndicator {
    pub fn from_position(
        position: &Position,
        date: Date,
        spot: &DataFrame,
        previous_value: &[PortfolioIndicator],
    ) -> PositionIndicator {
        debug!(
            "price position {} at {} with spot:{}",
            position.instrument.name,
            date,
            spot.close()
        );

        let (quantity, quantity_buy, quantity_sell, unit_price, tax) =
            Self::compute_quantity_(position, date);

        let valuation = spot.close() * quantity;
        let nominal = unit_price * quantity;

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

        let (current_pnl, daily_pnl, weekly_pnl, monthly_pnl, yearly_pnl) =
            make_pnls(date, nominal, valuation, |date, delta| {
                date.checked_sub_days(delta)
                    .and_then(|previous_day| {
                        previous_value.iter().rev().find(|item| {
                            item.date <= previous_day
                                && item.positions.iter().any(|item_postion| {
                                    item_postion.instrument == position.instrument
                                })
                        })
                    })
                    .and_then(|item| {
                        item.positions
                            .iter()
                            .find(|item_postion| item_postion.instrument == position.instrument)
                    })
                    .map(|item| (item.nominal, item.valuation))
            });

        let earning = dividends
            + position
                .trades
                .iter()
                .filter(|trade| trade.date.date() <= date)
                .fold(0.0, |earning, trade| {
                    let trade_price = match trade.way {
                        Way::Sell => trade.price * trade.quantity,
                        Way::Buy => -trade.price * trade.quantity,
                    };
                    trade_price + earning - trade.tax
                });
        let earning_latent = earning + valuation;

        PositionIndicator {
            spot: *spot,
            instrument: position.instrument.clone(),
            quantity,
            quantity_buy,
            quantity_sell,
            unit_price,
            valuation,
            nominal,
            dividends,
            tax,
            current_pnl,
            daily_pnl,
            weekly_pnl,
            monthly_pnl,
            yearly_pnl,
            earning,
            earning_latent,
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
    pub nominal: f64,
    pub dividends: f64,
    pub tax: f64,
    pub current_pnl: Pnl,
    pub daily_pnl: Pnl,
    pub weekly_pnl: Pnl,
    pub monthly_pnl: Pnl,
    pub yearly_pnl: Pnl,
    pub earning: f64,
    pub earning_latent: f64,
    pub cash: f64,
}

impl PortfolioIndicator {
    pub fn from_portfolio<P>(
        portfolio: &Portfolio,
        date: Date,
        spot_provider: &mut P,
        previous_value: &[PortfolioIndicator],
    ) -> PortfolioIndicator
    where
        P: Provider,
    {
        debug!("price portfolio at {}", date);
        let positions =
            PortfolioIndicator::make_positions_(portfolio, date, spot_provider, previous_value);

        let (valuation, nominal, dividends, tax, earning, earning_latent) = positions.iter().fold(
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            |(valuation, nominal, dividends, tax, earning, earning_latent), position_indicator| {
                (
                    valuation + position_indicator.valuation,
                    nominal + position_indicator.nominal,
                    dividends + position_indicator.dividends,
                    tax + position_indicator.tax,
                    earning + position_indicator.earning,
                    earning_latent + position_indicator.earning_latent,
                )
            },
        );

        let (current_pnl, daily_pnl, weekly_pnl, monthly_pnl, yearly_pnl) =
            make_pnls(date, nominal, valuation, |date, delta| {
                date.checked_sub_days(delta)
                    .and_then(|previous_day| {
                        previous_value
                            .iter()
                            .rev()
                            .find(|item| item.date <= previous_day)
                    })
                    .map(|item| (item.nominal, item.valuation))
            });

        let cash = portfolio
            .cash
            .iter()
            .filter(|variation| {
                variation.date.date() <= date && variation.source == CashVariationSource::Payment
            })
            .map(|variation| variation.position)
            .sum::<f64>()
            + positions
                .iter()
                .map(|position| position.earning)
                .sum::<f64>();

        PortfolioIndicator {
            date,
            positions,
            valuation,
            nominal,
            dividends,
            tax,
            current_pnl,
            daily_pnl,
            weekly_pnl,
            monthly_pnl,
            yearly_pnl,
            earning,
            earning_latent,
            cash,
        }
    }

    fn make_positions_<P>(
        portfolio: &Portfolio,
        date: Date,
        spot_provider: &mut P,
        previous_value: &[PortfolioIndicator],
    ) -> Vec<PositionIndicator>
    where
        P: Provider,
    {
        let mut data = Vec::new();
        for position in portfolio.positions.iter() {
            if !position
                .trades
                .first()
                .map(|trade| trade.date.date() <= date)
                .unwrap_or(false)
            {
                debug!(
                    "no pricing on {} at {} because of empty position",
                    position.instrument.name, date
                );
                continue;
            }
            if let Some(spot) = spot_provider.latest(&position.instrument, date) {
                let value = PositionIndicator::from_position(position, date, spot, previous_value);
                data.push(value);
            } else {
                error!(
                    "no spot on {} at {} and before skip position pricing",
                    position.instrument.name, date
                );
                data.clear();
                break;
            }
        }
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
                if instrument_begin < end {
                    spot_provider.fetch(&position.instrument, instrument_begin, end)?;
                }
            }
        }
        info!("request all market data historical done");

        info!("start to price portfolios");
        let portfolios =
            PortfolioIndicators::make_portfolios_(portfolio, begin, end, spot_provider);
        info!("price portfolios is finished");

        Ok(PortfolioIndicators { portfolios })
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
        let mut data = Vec::new();
        let mut it = begin;
        while it <= end {
            let value = PortfolioIndicator::from_portfolio(portfolio, it, spot_provider, &data);
            if !value.positions.is_empty() {
                data.push(value);
            } else {
                debug!("pricing result at {} is ignored (position empty)", it);
            }
            if let Some(next_it) = it.checked_add_days(chrono::naive::Days::new(1)) {
                it = next_it;
            } else {
                break;
            }
        }
        data
    }

    pub fn dump_position_indicators_in_csv(&self, filename: &str) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        output_stream.write_all(
            "Date;Cash;Valuation;Nominal;Dividends;Tax;P&L(%);P&L Daily(%);P&L Weekly(%),P&L Monthly(%);P&L Yearly(%);P&L;P&L Daily;P&L Weekly;P&L Monthly;P&L Yearly;Earning;Earning + Valuation\n".as_bytes(),
        )?;
        for portfolio_indicator in self.portfolios.iter() {
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

    pub fn dump_position_instrument_indicators_in_csv(
        &self,
        instrument_name: &str,
        filename: &str,
    ) -> Result<(), Error> {
        let mut output_stream = File::create(filename)?;
        output_stream.write_all(
            "Date;Instrument;Spot(Close);Quantity;Unit Price;Valuation;Nominal;Dividends;Tax;P&L(%);P&L Daily(%);P&L Weekly(%);P&L Monthly(%);P&L Yearly(%);P&L;P&L Daily;P&L Weekly;P&L Monthly;P&L Yearly;Earning;Earning + Valuation\n".as_bytes(),
        )?;
        for portfolio_indicator in self.portfolios.iter() {
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
