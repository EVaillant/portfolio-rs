use super::pnl::{make_pnls, Pnl};
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
    pub current_pnl: Pnl,
    pub daily_pnl: Pnl,
    pub weekly_pnl: Pnl,
    pub monthly_pnl: Pnl,
    pub yearly_pnl: Pnl,
    pub for_3_months_pnl: Pnl,
    pub for_1_year_pnl: Pnl,
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

        let (
            current_pnl,
            daily_pnl,
            weekly_pnl,
            monthly_pnl,
            yearly_pnl,
            for_3_months_pnl,
            for_1_year_pnl,
        ) = make_pnls(date, nominal, valuation, |date, delta| {
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
            current_pnl,
            daily_pnl,
            weekly_pnl,
            monthly_pnl,
            yearly_pnl,
            for_3_months_pnl,
            for_1_year_pnl,
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
