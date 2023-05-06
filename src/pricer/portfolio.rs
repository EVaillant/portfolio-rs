use super::pnl::{make_pnls, Pnl};
use super::position::PositionIndicator;
use crate::alias::Date;
use crate::historical::Provider;
use crate::portfolio::{CashVariationSource, Portfolio};

use log::{debug, error};

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
