use super::tools::{make_pnls, make_volatilities, Pnl};
use super::PortfolioIndicator;
use crate::alias::Date;
use crate::historical::DataFrame;
use crate::marketdata::Instrument;
use crate::portfolio::{Position, Way};
use std::rc::Rc;

use log::debug;

pub struct PositionIndicator {
    pub date: Date,
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
    pub pnl_current: Pnl,
    pub pnl_daily: Pnl,
    pub pnl_weekly: Pnl,
    pub pnl_monthly: Pnl,
    pub pnl_yearly: Pnl,
    pub pnl_for_3_months: Pnl,
    pub pnl_for_1_year: Pnl,
    pub volatility_3_month: f64,
    pub volatility_1_year: f64,
    pub earning: f64,
    pub earning_latent: f64,
    pub is_already_close: bool,
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

        let (
            pnl_current,
            pnl_daily,
            pnl_weekly,
            pnl_monthly,
            pnl_yearly,
            pnl_for_3_months,
            pnl_for_1_year,
        ) = make_pnls(date, nominal, valuation, |date| {
            previous_value
                .iter()
                .find(|item| {
                    item.date >= date
                        && item
                            .positions
                            .iter()
                            .any(|item_postion| item_postion.instrument == position.instrument)
                })
                .and_then(|item| {
                    item.positions
                        .iter()
                        .find(|item_postion| item_postion.instrument == position.instrument)
                })
                .map(|item| (item.nominal, item.valuation))
        });

        let (volatility_3_month, volatility_1_year) = make_volatilities(date, |date| {
            let mut ret = previous_value
                .iter()
                .filter(|item| item.date >= date)
                .map(|item| {
                    item.positions
                        .iter()
                        .find(|item_position| item_position.instrument == position.instrument)
                })
                .filter(Option::is_some)
                .map(|item| item.unwrap().pnl_current.value_pct)
                .collect::<Vec<_>>();
            ret.push(pnl_current.value_pct);
            ret
        });

        let is_already_close = quantity.abs() < 1e-7
            && position
                .trades
                .last()
                .map(|trade| trade.date.date() < date)
                .unwrap_or(false);

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
            date,
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
            pnl_current,
            pnl_daily,
            pnl_weekly,
            pnl_monthly,
            pnl_yearly,
            pnl_for_3_months,
            pnl_for_1_year,
            volatility_3_month,
            volatility_1_year,
            earning,
            earning_latent,
            is_already_close,
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
                            if quantity.abs() < 1e-7 {
                                quantity = 0.0;
                                unit_price = 0.0;
                            }
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
