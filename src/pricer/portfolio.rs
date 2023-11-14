use super::position::PositionIndicator;
use super::tools::{Pnl, PnlAccumulator};
use crate::alias::Date;
use crate::historical::Provider;
use crate::portfolio::{CashVariationSource, Portfolio};
use std::collections::HashMap;

use log::{debug, error};
#[derive(Default)]
pub struct PortfolioIndicator {
    pub date: Date,
    pub positions: Vec<PositionIndicator>,
    pub valuation: f64,
    pub nominal: f64,
    pub cashflow: f64,
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
    pub incoming_transfer: f64,
    pub outcoming_transfer: f64,
    pub cash: f64,
}

impl PortfolioIndicator {
    pub fn from_portfolio<P>(
        portfolio: &Portfolio,
        date: Date,
        spot_provider: &mut P,
        pnl_accumulator: &mut PnlAccumulator,
        pnl_accumulator_by_instrument: &mut HashMap<String, PnlAccumulator>,
    ) -> PortfolioIndicator
    where
        P: Provider,
    {
        debug!("price portfolio at {}", date);
        let positions = PortfolioIndicator::make_positions_(
            portfolio,
            date,
            spot_provider,
            pnl_accumulator_by_instrument,
        );

        let (valuation, nominal, cashflow, dividends, tax, earning, earning_latent) =
            positions.iter().fold(
                (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
                |(valuation, nominal, cashflow, dividends, tax, earning, earning_latent),
                 position_indicator| {
                    (
                        valuation + position_indicator.valuation,
                        nominal + position_indicator.nominal,
                        cashflow + position_indicator.cashflow,
                        dividends + position_indicator.dividends,
                        tax + position_indicator.tax,
                        earning + position_indicator.earning,
                        earning_latent + position_indicator.earning_latent,
                    )
                },
            );

        pnl_accumulator.append(date, cashflow, valuation);

        let incoming_transfer = portfolio
            .cash
            .iter()
            .filter(|variation| {
                variation.date.date() <= date
                    && variation.source == CashVariationSource::Payment
                    && variation.position.is_sign_positive()
            })
            .map(|variation| variation.position)
            .sum::<f64>();

        let outcoming_transfer = portfolio
            .cash
            .iter()
            .filter(|variation| {
                variation.date.date() <= date
                    && variation.source == CashVariationSource::Payment
                    && variation.position.is_sign_negative()
            })
            .map(|variation| variation.position)
            .sum::<f64>();

        let cash = incoming_transfer
            + outcoming_transfer
            + positions
                .iter()
                .map(|position| position.earning)
                .sum::<f64>();

        PortfolioIndicator {
            date,
            positions: positions
                .into_iter()
                .filter(|position| !position.is_already_close)
                .collect(),
            valuation,
            nominal,
            cashflow,
            dividends,
            tax,
            pnl_current: pnl_accumulator.global,
            pnl_daily: pnl_accumulator.daily,
            pnl_weekly: pnl_accumulator.weekly,
            pnl_monthly: pnl_accumulator.monthly,
            pnl_yearly: pnl_accumulator.yearly,
            pnl_for_3_months: pnl_accumulator.for_3_months,
            pnl_for_1_year: pnl_accumulator.for_1_year,
            volatility_3_month: pnl_accumulator.volatility_3_month,
            volatility_1_year: pnl_accumulator.volatility_1_year,
            earning,
            earning_latent,
            incoming_transfer,
            outcoming_transfer,
            cash,
        }
    }

    pub fn make_distribution_by_region(&self) -> HashMap<String, f64> {
        let mut ret: HashMap<String, f64> = Default::default();
        for position in self.positions.iter() {
            let value = ret
                .entry(position.instrument.region.clone())
                .or_insert_with(|| 0.0);
            *value += position.valuation / self.valuation;
        }
        ret
    }

    pub fn make_distribution_by_instrument(&self, region_name: &str) -> HashMap<String, f64> {
        let mut ret: HashMap<String, f64> = Default::default();
        let mut valuation = 0.0;
        for position in self
            .positions
            .iter()
            .filter(|item| item.instrument.region == region_name)
        {
            if position.valuation.abs() < 1e-7 {
                continue;
            }
            valuation += position.valuation;
            *ret.entry(position.instrument.name.clone())
                .or_insert_with(|| 0.0) = position.valuation;
        }
        ret.iter_mut().for_each(|(_, value)| {
            *value /= valuation;
        });
        ret
    }

    pub fn make_distribution_global_by_instrument(&self) -> HashMap<String, f64> {
        let mut ret: HashMap<String, f64> = Default::default();
        for position in self.positions.iter() {
            if position.valuation.abs() < 1e-7 {
                continue;
            }
            let value = ret
                .entry(position.instrument.name.clone())
                .or_insert_with(|| 0.0);
            *value += position.valuation / self.valuation;
        }
        ret
    }

    fn make_positions_<P>(
        portfolio: &Portfolio,
        date: Date,
        spot_provider: &mut P,
        pnl_accumulator_by_instrument: &mut HashMap<String, PnlAccumulator>,
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
                let value = PositionIndicator::from_position(
                    position,
                    date,
                    spot,
                    pnl_accumulator_by_instrument,
                );
                if !value.is_already_close {
                    data.push(value);
                }
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
