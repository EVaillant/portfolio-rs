use super::position::PositionIndicator;
use super::tools::{make_pnls, make_volatilities, Pnl};
use crate::alias::Date;
use crate::historical::Provider;
use crate::portfolio::{CashVariationSource, Portfolio};
use std::collections::HashMap;

use log::{debug, error};

pub struct PortfolioIndicator {
    pub date: Date,
    pub positions: Vec<PositionIndicator>,
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
                .find(|item| item.date >= date)
                .map(|item| (item.nominal, item.valuation))
        });

        let (volatility_3_month, volatility_1_year) = make_volatilities(date, |date| {
            let mut ret = previous_value
                .iter()
                .filter(|item| item.date >= date)
                .map(|item| item.pnl_current.value_pct)
                .collect::<Vec<_>>();
            ret.push(pnl_current.value_pct);
            ret
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
