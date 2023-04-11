use crate::alias::Date;
use crate::error::Error;
use crate::historical::{DataFrame, Provider};
use crate::marketdata::Instrument;
use crate::portfolio::{Portfolio, Position, Way};
use chrono::naive::Days;
use chrono::Datelike;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;

use log::{debug, info};

pub struct Pnl {
    pub beginning: f64,
    pub daily: f64,
    pub weekly: f64,
    pub monthly: f64,
    pub yearly: f64,
}

impl Pnl {
    pub fn new(beginning: f64, daily: f64, weekly: f64, monthly: f64, yearly: f64) -> Self {
        Self {
            beginning,
            daily: Self::compute_(beginning, daily),
            weekly: Self::compute_(beginning, weekly),
            monthly: Self::compute_(beginning, monthly),
            yearly: Self::compute_(beginning, yearly),
        }
    }

    fn compute_(current: f64, last: f64) -> f64 {
        (current + 1.0) / (last + 1.0) - 1.0
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
    pub nominal: f64,
    pub dividends: f64,
    pub tax: f64,
    pub pnl: Pnl,
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

        let pnl = if quantity == 0.0 {
            Pnl::new(0.0, 0.0, 0.0, 0.0, 0.0)
        } else {
            let (daily, weekly, monthly, yearly) =
                Self::previous_pnl_(date, position, previous_value);
            Pnl::new(
                (valuation - nominal) / nominal,
                daily,
                weekly,
                monthly,
                yearly,
            )
        };
        let earning = position
            .trades
            .iter()
            .filter(|trade| trade.date.date() <= date)
            .fold(dividends, |earning, trade| {
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
            pnl,
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

    fn previous_pnl_(
        date: Date,
        position: &Position,
        previous_value: &[PortfolioIndicator],
    ) -> (f64, f64, f64, f64) {
        let previous_day = Self::get_previous_pnl_(date, Days::new(1), position, previous_value);
        let previous_week = Self::get_previous_pnl_(
            date,
            Days::new((date.weekday().num_days_from_monday() + 1) as u64),
            position,
            previous_value,
        );
        let previous_month =
            Self::get_previous_pnl_(date, Days::new(date.day() as u64), position, previous_value);
        let previous_year =
            Date::from_ymd_opt(date.year() - 1, 12, 31).and_then(|previous_year_date| {
                Self::get_previous_pnl_(previous_year_date, Days::new(0), position, previous_value)
            });

        (
            previous_day.unwrap_or(0.0),
            previous_week.unwrap_or(0.0),
            previous_month.unwrap_or(0.0),
            previous_year.unwrap_or(0.0),
        )
    }

    fn get_previous_pnl_(
        date: Date,
        delta: Days,
        position: &Position,
        previous_value: &[PortfolioIndicator],
    ) -> Option<f64> {
        date.checked_sub_days(delta)
            .and_then(|previous_day| {
                previous_value.iter().rev().find(|item| {
                    item.date <= previous_day
                        && item
                            .positions
                            .iter()
                            .any(|item_postion| item_postion.instrument == position.instrument)
                })
            })
            .and_then(|item| {
                item.positions
                    .iter()
                    .find(|item_postion| item_postion.instrument == position.instrument)
            })
            .map(|item| item.pnl.beginning)
    }
}

pub struct PortfolioIndicator {
    pub date: Date,
    pub positions: Vec<PositionIndicator>,
    pub valuation: f64,
    pub nominal: f64,
    pub dividends: f64,
    pub tax: f64,
    pub pnl: Pnl,
    pub earning: f64,
    pub earning_latent: f64,
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

        let pnl = if nominal == 0.0 {
            Pnl::new(0.0, 0.0, 0.0, 0.0, 0.0)
        } else {
            let (daily, weekly, monthly, yearly) = Self::previous_pnl_(date, previous_value);
            Pnl::new(
                (valuation - nominal) / nominal,
                daily,
                weekly,
                monthly,
                yearly,
            )
        };

        PortfolioIndicator {
            date,
            positions,
            valuation,
            nominal,
            dividends,
            tax,
            pnl,
            earning,
            earning_latent,
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
            if let Some(spot) = spot_provider.get(&position.instrument, date) {
                let value = PositionIndicator::from_position(position, date, spot, previous_value);
                data.push(value);
            } else {
                debug!(
                    "no spot on {} at {} skip position pricing",
                    position.instrument.name, date
                );
                data.clear();
                break;
            }
        }
        data
    }

    fn previous_pnl_(date: Date, previous_value: &[PortfolioIndicator]) -> (f64, f64, f64, f64) {
        let previous_day = Self::get_previous_pnl_(date, Days::new(1), previous_value);
        let previous_week = Self::get_previous_pnl_(
            date,
            Days::new((date.weekday().num_days_from_monday() + 1) as u64),
            previous_value,
        );
        let previous_month =
            Self::get_previous_pnl_(date, Days::new(date.day() as u64), previous_value);
        let previous_year =
            Date::from_ymd_opt(date.year() - 1, 12, 31).and_then(|previous_year_date| {
                Self::get_previous_pnl_(previous_year_date, Days::new(0), previous_value)
            });

        (
            previous_day.unwrap_or(0.0),
            previous_week.unwrap_or(0.0),
            previous_month.unwrap_or(0.0),
            previous_year.unwrap_or(0.0),
        )
    }

    fn get_previous_pnl_(
        date: Date,
        delta: Days,
        previous_value: &[PortfolioIndicator],
    ) -> Option<f64> {
        date.checked_sub_days(delta)
            .and_then(|previous_day| {
                previous_value
                    .iter()
                    .rev()
                    .find(|item| item.date <= previous_day)
            })
            .map(|item| item.pnl.beginning)
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
            "Date;Valuation;Nominal;Dividends;Tax;P&L(%);P&L Daily(%);P&L Weekly(%),P&L Monthly(%);P&L Yearly(%);P&L;P&L Daily;P&L Weekly;P&L Monthly;P&L Yearly;Earning;Earning + Valuation\n".as_bytes(),
        )?;
        self.portfolios.iter().for_each(|portfolio_indicator| {
            output_stream
                .write_all(
                    format!(
                        "{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}\n",
                        portfolio_indicator.date.format("%Y-%m-%d"),
                        portfolio_indicator.valuation,
                        portfolio_indicator.nominal,
                        portfolio_indicator.dividends,
                        portfolio_indicator.tax,
                        portfolio_indicator.pnl.beginning,
                        portfolio_indicator.pnl.daily,
                        portfolio_indicator.pnl.weekly,
                        portfolio_indicator.pnl.monthly,
                        portfolio_indicator.pnl.yearly,
                        portfolio_indicator.pnl.beginning * portfolio_indicator.nominal,
                        portfolio_indicator.pnl.daily * portfolio_indicator.nominal,
                        portfolio_indicator.pnl.weekly * portfolio_indicator.nominal,
                        portfolio_indicator.pnl.monthly * portfolio_indicator.nominal,
                        portfolio_indicator.pnl.yearly * portfolio_indicator.nominal,
                        portfolio_indicator.earning,
                        portfolio_indicator.earning_latent
                    )
                    .as_bytes(),
                )
                .unwrap();
        });
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
                            "{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}\n",
                            position_indicator.spot.date().format("%Y-%m-%d"),
                            instrument_name,
                            position_indicator.spot.close(),
                            position_indicator.quantity,
                            position_indicator.unit_price,
                            position_indicator.valuation,
                            position_indicator.nominal,
                            position_indicator.dividends,
                            position_indicator.tax,
                            position_indicator.pnl.beginning,
                            position_indicator.pnl.daily,
                            position_indicator.pnl.weekly,
                            position_indicator.pnl.monthly,
                            position_indicator.pnl.yearly,
                            position_indicator.pnl.beginning * position_indicator.nominal,
                            position_indicator.pnl.daily * position_indicator.nominal,
                            position_indicator.pnl.weekly * position_indicator.nominal,
                            position_indicator.pnl.monthly * position_indicator.nominal,
                            position_indicator.pnl.yearly * position_indicator.nominal,
                            position_indicator.earning,
                            position_indicator.earning_latent,
                        )
                        .as_bytes(),
                    )
                    .unwrap();
            });
        Ok(())
    }
}
