use super::position::PositionIndicator;
use super::primitive;
use crate::alias::{Date, Duration};
use crate::portfolio::{CashVariationSource, Portfolio};
use std::iter::Sum;
use std::ops::Add;

use log::debug;

#[derive(Default)]
struct PositionAccumulator {
    pub valuation: f64,
    pub nominal: f64,
    pub dividends: f64,
    pub fees: f64,
    pub earning: f64,
    pub earning_latent: f64,
}

impl PositionAccumulator {
    fn from_position(position: &PositionIndicator) -> Self {
        Self::from_position_(position)
    }

    fn from_open_position(position: &PositionIndicator) -> Self {
        if position.is_close {
            Default::default()
        } else {
            Self::from_position_(position)
        }
    }

    fn from_position_(position: &PositionIndicator) -> Self {
        Self {
            valuation: position.valuation,
            nominal: position.nominal,
            dividends: position.dividends,
            fees: position.fees,
            earning: position.earning,
            earning_latent: position.earning_latent,
        }
    }
}

impl Add for PositionAccumulator {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            valuation: other.valuation + self.valuation,
            nominal: other.nominal + self.nominal,
            dividends: other.dividends + self.dividends,
            fees: other.fees + self.fees,
            earning: other.earning + self.earning,
            earning_latent: other.earning_latent + self.earning_latent,
        }
    }
}

impl Sum for PositionAccumulator {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Default::default(), PositionAccumulator::add)
    }
}

#[derive(Default)]
pub struct PortfolioIndicator {
    pub date: Date,
    pub positions: Vec<PositionIndicator>,
    pub valuation: f64,
    pub open_valuation: f64,
    pub nominal: f64,
    pub open_nominal: f64,
    pub dividends: f64,
    pub open_dividends: f64,
    pub fees: f64,
    pub open_fees: f64,
    pub fees_percent: f64,
    pub pnl_currency: f64,
    pub pnl_percent: f64,
    pub pnl_volatility_3m: f64,
    pub pnl_actuarial: f64,
    pub twr: f64,
    pub open_pnl_currency: f64,
    pub open_pnl_percent: f64,
    pub open_pnl_volatility_3m: f64,
    pub open_twr: f64,
    pub earning: f64,
    pub open_earning: f64,
    pub earning_latent: f64,
    pub open_earning_latent: f64,
    pub incoming_transfer: f64,
    pub outcoming_transfer: f64,
    pub cash: f64,
}

impl PortfolioIndicator {
    pub fn from_portfolio(
        portfolio: &Portfolio,
        date: Date,
        positions: Vec<PositionIndicator>,
        previous_indicators: &[PortfolioIndicator],
    ) -> PortfolioIndicator {
        debug!("price portfolio at {date}");

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

        let other_transfer = portfolio
            .cash
            .iter()
            .filter(|variation| {
                variation.date.date() <= date && variation.source != CashVariationSource::Payment
            })
            .map(|variation| variation.position)
            .sum::<f64>();

        let accumulator = positions
            .iter()
            .map(PositionAccumulator::from_position)
            .sum::<PositionAccumulator>();

        let open_accumulator = positions
            .iter()
            .map(PositionAccumulator::from_open_position)
            .sum::<PositionAccumulator>();

        let cash = outcoming_transfer + incoming_transfer + other_transfer + accumulator.earning;
        let valuation = cash + accumulator.valuation;
        let nominal = Self::compute_nominal_(portfolio, date, previous_indicators, valuation);

        let fees_percent = if valuation + accumulator.fees == 0.0 {
            0.0
        } else {
            accumulator.fees / (accumulator.fees + valuation)
        };
        let open_nominal = open_accumulator.nominal;
        let open_valuation = open_accumulator.valuation;
        let (pnl_currency, pnl_percent) = primitive::pnl(valuation, nominal);
        let (open_pnl_currency, open_pnl_percent) = primitive::pnl(open_valuation, open_nominal);
        let pnl_actuarial = primitive::actuarial_rate(&portfolio.open, &date, pnl_percent);

        let pnl_volatility_3m = primitive::volatility_from(
            date,
            Duration::days(90),
            previous_indicators,
            pnl_percent,
            |item| item.pnl_percent,
            |item| item.date,
        );

        let open_pnl_volatility_3m = primitive::volatility_from(
            date,
            Duration::days(90),
            previous_indicators,
            open_pnl_percent,
            |item| item.open_pnl_percent,
            |item| item.date,
        );

        let (previous_twr, begin_valuation, delta_cashflow) =
            if let Some(previous_indicator) = previous_indicators.last() {
                (
                    previous_indicator.twr,
                    previous_indicator.valuation,
                    nominal - previous_indicator.nominal,
                )
            } else {
                (0.0, nominal, 0.0)
            };
        let twr = primitive::twr(begin_valuation, valuation, delta_cashflow, previous_twr);

        let (previous_twr, begin_valuation, delta_cashflow) =
            if let Some(previous_indicator) = previous_indicators.last() {
                (
                    previous_indicator.open_twr,
                    previous_indicator.open_valuation,
                    open_nominal - previous_indicator.open_nominal,
                )
            } else {
                (0.0, open_nominal, 0.0)
            };
        let open_twr = primitive::twr(
            begin_valuation,
            open_valuation,
            delta_cashflow,
            previous_twr,
        );

        PortfolioIndicator {
            date,
            positions,
            valuation,
            open_valuation,
            nominal,
            open_nominal,
            dividends: accumulator.dividends,
            open_dividends: open_accumulator.dividends,
            fees: accumulator.fees,
            open_fees: open_accumulator.fees,
            fees_percent,
            pnl_currency,
            pnl_percent,
            pnl_actuarial,
            pnl_volatility_3m,
            open_pnl_currency,
            open_pnl_percent,
            open_pnl_volatility_3m,
            twr,
            open_twr,
            earning: accumulator.earning,
            open_earning: open_accumulator.earning,
            earning_latent: accumulator.earning_latent,
            open_earning_latent: open_accumulator.earning_latent,
            incoming_transfer,
            outcoming_transfer,
            cash,
        }
    }

    fn compute_nominal_(
        portfolio: &Portfolio,
        date: Date,
        previous_indicators: &[PortfolioIndicator],
        valuation: f64,
    ) -> f64 {
        let previous_nominal = previous_indicators
            .last()
            .map(|previous| previous.nominal)
            .unwrap_or(0.0);
        let previous_date = previous_indicators.last().map(|previous| previous.date);

        let incoming_transfer = portfolio
            .cash
            .iter()
            .filter(|variation| {
                previous_date
                    .map(|value| variation.date.date() > value)
                    .unwrap_or(true)
                    && variation.date.date() <= date
                    && variation.source == CashVariationSource::Payment
                    && variation.position.is_sign_positive()
            })
            .map(|variation| variation.position)
            .sum::<f64>();
        let outcoming_transfer = portfolio
            .cash
            .iter()
            .filter(|variation| {
                previous_date
                    .map(|value| variation.date.date() > value)
                    .unwrap_or(true)
                    && variation.date.date() <= date
                    && variation.source == CashVariationSource::Payment
                    && variation.position.is_sign_negative()
            })
            .map(|variation| variation.position)
            .sum::<f64>();

        if outcoming_transfer == 0.0 {
            previous_nominal + incoming_transfer
        } else {
            let nominal_without_output = previous_nominal + incoming_transfer;
            let pnl_percent =
                primitive::pnl(valuation - outcoming_transfer, nominal_without_output).1;
            valuation / (pnl_percent + 1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::historical::DataFrame;
    use crate::marketdata::{Currency, Instrument, Market};
    use crate::portfolio::{CashVariation, CashVariationSource};
    use assert_float_eq::*;
    use std::rc::Rc;

    fn make_fake_position_indicator_(
        valuation: f64,
        nominal: f64,
        dividends: f64,
        earning: f64,
        earning_latent: f64,
        fees: f64,
    ) -> PositionIndicator {
        let date = chrono::NaiveDate::from_ymd_opt(2025, 3, 17).unwrap();
        let currency = Rc::new(Currency {
            name: String::from("EUR"),
            parent_currency: None,
        });
        let market = Rc::new(Market {
            name: String::from("EPA"),
            description: String::from("EPA"),
        });
        let instrument = Rc::new(Instrument {
            name: String::from("PAEEM"),
            isin: String::from("ISIN"),
            description: String::from("description"),
            market: market.clone(),
            currency: currency.clone(),
            ticker_yahoo: None,
            region: None,
            fund_category: String::from("category"),
            dividends: None,
        });
        PositionIndicator {
            date,
            spot: DataFrame::new(date, 22.0, 22.0, 22.0, 22.0),
            instrument,
            position_index: 0,
            quantity: 0.0,
            quantity_buy: 0.0,
            quantity_sell: 0.0,
            unit_price: 0.0,
            valuation,
            nominal,
            cashflow: 0.0,
            dividends,
            fees,
            pnl_currency: 0.0,
            pnl_percent: 0.0,
            pnl_volatility_3m: 0.0,
            twr: 0.0,
            irr: None,
            earning,
            earning_latent,
            is_close: false,
        }
    }

    #[test]
    fn compute_portfolio() {
        let currency = Rc::new(Currency {
            name: String::from("EUR"),
            parent_currency: None,
        });

        let portfolio = Portfolio {
            name: "TEST".to_string(),
            open: Date::from_ymd_opt(2022, 3, 17).unwrap(),
            incoming_transfer_limit: None,
            currency: currency.clone(),
            positions: Default::default(),
            cash: vec![CashVariation {
                position: 1000.0,
                date: chrono::DateTime::parse_from_rfc3339("2022-03-17T10:00:00-00:00")
                    .unwrap()
                    .naive_local(),
                source: CashVariationSource::Payment,
            }],
        };

        let mut previous_indicators = Vec::new();
        {
            let date = chrono::NaiveDate::from_ymd_opt(2025, 3, 17).unwrap();
            let positions_indicators = vec![make_fake_position_indicator_(
                200.0, 190.0, 0.0, -190.0, -190.0, 2.0,
            )];

            let indicator = PortfolioIndicator::from_portfolio(
                &portfolio,
                date,
                positions_indicators,
                &previous_indicators,
            );

            assert_float_absolute_eq!(indicator.incoming_transfer, 1000.0, 1e-7);
            assert_float_absolute_eq!(indicator.outcoming_transfer, 0.0, 1e-7);
            assert_float_absolute_eq!(indicator.nominal, 1000.0, 1e-7);
            assert_float_absolute_eq!(indicator.cash, 810.0, 1e-7);
            assert_float_absolute_eq!(indicator.valuation, 1010.0, 1e-7);
            assert_float_absolute_eq!(indicator.fees, 2.0, 1e-7);
            assert_float_absolute_eq!(indicator.dividends, 0.0, 1e-7);
            assert_float_absolute_eq!(indicator.earning, -190.0, 1e-7);
            assert_float_absolute_eq!(indicator.earning_latent, -190.0, 1e-7);
            assert_float_absolute_eq!(indicator.pnl_currency, 10.0, 1e-7);
            assert_float_absolute_eq!(indicator.pnl_percent, 0.01, 1e-7);
            assert_float_absolute_eq!(indicator.twr, 0.01, 1e-7);

            previous_indicators.push(indicator);
        }
        {
            let date = chrono::NaiveDate::from_ymd_opt(2025, 3, 18).unwrap();
            let positions_indicators = vec![
                make_fake_position_indicator_(300.0, 190.0, 0.0, -190.0, -190.0, 2.0),
                make_fake_position_indicator_(500.0, 400.0, 0.0, -400.0, -400.0, 5.0),
            ];

            let indicator = PortfolioIndicator::from_portfolio(
                &portfolio,
                date,
                positions_indicators,
                &previous_indicators,
            );

            assert_float_absolute_eq!(indicator.incoming_transfer, 1000.0, 1e-7);
            assert_float_absolute_eq!(indicator.outcoming_transfer, 0.0, 1e-7);
            assert_float_absolute_eq!(indicator.nominal, 1000.0, 1e-7);
            assert_float_absolute_eq!(indicator.cash, 410.0, 1e-7);
            assert_float_absolute_eq!(indicator.valuation, 1210.0, 1e-7);
            assert_float_absolute_eq!(indicator.fees, 7.0, 1e-7);
            assert_float_absolute_eq!(indicator.dividends, 0.0, 1e-7);
            assert_float_absolute_eq!(indicator.earning, -590.0, 1e-7);
            assert_float_absolute_eq!(indicator.earning_latent, -590.0, 1e-7);
            assert_float_absolute_eq!(indicator.pnl_currency, 210.0, 1e-7);
            assert_float_absolute_eq!(indicator.pnl_percent, 0.21, 1e-7);
            assert_float_absolute_eq!(indicator.twr, 0.21, 1e-7);

            previous_indicators.push(indicator);
        }
    }

    #[test]
    fn compute_nominal() {
        let currency = Rc::new(Currency {
            name: String::from("EUR"),
            parent_currency: None,
        });

        let portfolio = Portfolio {
            name: "TEST".to_string(),
            open: Date::from_ymd_opt(2022, 3, 17).unwrap(),
            incoming_transfer_limit: None,
            currency: currency.clone(),
            positions: Default::default(),
            cash: vec![
                CashVariation {
                    position: 1000.0,
                    date: chrono::DateTime::parse_from_rfc3339("2022-03-17T10:00:00-00:00")
                        .unwrap()
                        .naive_local(),
                    source: CashVariationSource::Payment,
                },
                CashVariation {
                    position: 1500.0,
                    date: chrono::DateTime::parse_from_rfc3339("2022-04-17T10:00:00-00:00")
                        .unwrap()
                        .naive_local(),
                    source: CashVariationSource::Payment,
                },
                CashVariation {
                    position: -100.0,
                    date: chrono::DateTime::parse_from_rfc3339("2022-05-17T10:00:00-00:00")
                        .unwrap()
                        .naive_local(),
                    source: CashVariationSource::Payment,
                },
            ],
        };

        let mut previous_indicators = Vec::new();
        {
            let nominal = PortfolioIndicator::compute_nominal_(
                &portfolio,
                chrono::NaiveDate::from_ymd_opt(2022, 3, 17).unwrap(),
                &previous_indicators,
                1500.0,
            );
            assert_float_absolute_eq!(nominal, 1000.0, 1e-7);

            let nominal = PortfolioIndicator::compute_nominal_(
                &portfolio,
                chrono::NaiveDate::from_ymd_opt(2022, 3, 20).unwrap(),
                &previous_indicators,
                1500.0,
            );
            assert_float_absolute_eq!(nominal, 1000.0, 1e-7);

            let nominal = PortfolioIndicator::compute_nominal_(
                &portfolio,
                chrono::NaiveDate::from_ymd_opt(2022, 4, 17).unwrap(),
                &previous_indicators,
                4000.0,
            );
            assert_float_absolute_eq!(nominal, 2500.0, 1e-7);

            let nominal = PortfolioIndicator::compute_nominal_(
                &portfolio,
                chrono::NaiveDate::from_ymd_opt(2022, 4, 20).unwrap(),
                &previous_indicators,
                4000.0,
            );
            assert_float_absolute_eq!(nominal, 2500.0, 1e-7);
        }

        let indicator = PortfolioIndicator {
            date: chrono::NaiveDate::from_ymd_opt(2022, 3, 18).unwrap(),
            nominal: 1000.0,
            ..Default::default()
        };
        previous_indicators.push(indicator);
        {
            let nominal = PortfolioIndicator::compute_nominal_(
                &portfolio,
                chrono::NaiveDate::from_ymd_opt(2022, 3, 17).unwrap(),
                &previous_indicators,
                1500.0,
            );
            assert_float_absolute_eq!(nominal, 1000.0, 1e-7);

            let nominal = PortfolioIndicator::compute_nominal_(
                &portfolio,
                chrono::NaiveDate::from_ymd_opt(2022, 3, 20).unwrap(),
                &previous_indicators,
                1500.0,
            );
            assert_float_absolute_eq!(nominal, 1000.0, 1e-7);

            let nominal = PortfolioIndicator::compute_nominal_(
                &portfolio,
                chrono::NaiveDate::from_ymd_opt(2022, 4, 17).unwrap(),
                &previous_indicators,
                4000.0,
            );
            assert_float_absolute_eq!(nominal, 2500.0, 1e-7);

            let nominal = PortfolioIndicator::compute_nominal_(
                &portfolio,
                chrono::NaiveDate::from_ymd_opt(2022, 4, 20).unwrap(),
                &previous_indicators,
                4000.0,
            );
            assert_float_absolute_eq!(nominal, 2500.0, 1e-7);
        }

        let indicator = PortfolioIndicator {
            date: chrono::NaiveDate::from_ymd_opt(2022, 4, 20).unwrap(),
            nominal: 2500.0,
            ..Default::default()
        };
        previous_indicators.push(indicator);
        {
            let nominal = PortfolioIndicator::compute_nominal_(
                &portfolio,
                chrono::NaiveDate::from_ymd_opt(2022, 5, 17).unwrap(),
                &previous_indicators,
                5000.0,
            );
            assert_float_absolute_eq!(nominal, 2450.9803921568628, 1e-7);
        }
    }
}
