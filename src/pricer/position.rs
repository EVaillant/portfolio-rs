use super::tools::{Pnl, PnlAccumulator};
use crate::alias::Date;
use crate::historical::DataFrame;
use crate::marketdata::Instrument;
use crate::portfolio::{Position, Way};
use std::collections::HashMap;
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
    pub is_already_close: bool,
}

impl PositionIndicator {
    pub fn from_position(
        position: &Position,
        date: Date,
        spot: &DataFrame,
        pnl_accumulator_by_position: &mut HashMap<String, PnlAccumulator>,
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

        let cashflow = position
            .trades
            .iter()
            .filter(|trade| trade.date.date() == date)
            .map(|trade| {
                let way = match trade.way {
                    Way::Sell => -1.0,
                    Way::Buy => 1.0,
                };
                way * trade.quantity * trade.price + trade.tax
            })
            .sum();

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

        let is_already_close = quantity.abs() < 1e-7
            && !position
                .trades
                .iter()
                .any(|trade| trade.date.date() == date);

        let pnl_accumulator = pnl_accumulator_by_position
            .entry(position.instrument.name.to_string())
            .or_default();
        pnl_accumulator.append(date, cashflow, valuation);
        dbg!(
            date,
            position.instrument.name.to_string(),
            cashflow,
            valuation
        );

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marketdata::{Currency, Instrument, Market};
    use crate::portfolio::{Position, Trade, Way};
    use crate::pricer::PortfolioIndicator;
    use assert_float_eq::*;

    fn make_instrument_(name: &str) -> Rc<Instrument> {
        let currency = Rc::new(Currency {
            name: String::from("EUR"),
            parent_currency: None,
        });

        let market = Rc::new(Market {
            name: String::from("EPA"),
            description: String::from("EPA"),
        });

        Rc::new(Instrument {
            name: String::from(name),
            isin: String::from("ISIN"),
            description: String::from("description"),
            market,
            currency,
            ticker_yahoo: None,
            region: String::from("region"),
            fund_category: String::from("category"),
            dividends: None,
        })
    }

    fn make_indicator_(
        position: &Position,
        date: Date,
        spot: f64,
        pnl_accumulator_by_instrument: &mut HashMap<String, PnlAccumulator>,
    ) -> PositionIndicator {
        let spot = DataFrame::new(date, spot, spot, spot, spot);
        PositionIndicator::from_position(position, date, &spot, pnl_accumulator_by_instrument)
    }

    fn make_default_portfolio_indicator_(
        position_indicator: PositionIndicator,
    ) -> PortfolioIndicator {
        PortfolioIndicator {
            date: position_indicator.date,
            positions: vec![position_indicator],
            ..Default::default()
        }
    }

    fn check_indicator_(
        indicator: &PositionIndicator,
        valuation: f64,
        nominal: f64,
        cashflow: f64,
        quantity: (f64, f64, f64),
        unit_price: f64,
        previous_valuations: (f64, f64, f64, f64),
    ) {
        assert_float_absolute_eq!(indicator.valuation, valuation, 1e-7);
        assert_float_absolute_eq!(indicator.nominal, nominal, 1e-7);
        assert_float_absolute_eq!(indicator.cashflow, cashflow, 1e-7);
        assert_float_absolute_eq!(indicator.quantity, quantity.0, 1e-7);
        assert_float_absolute_eq!(indicator.quantity_buy, quantity.1, 1e-7);
        assert_float_absolute_eq!(indicator.quantity_sell, quantity.2, 1e-7);
        assert_float_absolute_eq!(indicator.unit_price, unit_price, 1e-7);
        assert_float_absolute_eq!(
            indicator.pnl_daily.value,
            indicator.valuation - (cashflow + previous_valuations.0),
            1e-7
        );
        assert_float_absolute_eq!(
            indicator.pnl_current.value,
            indicator.pnl_daily.value + previous_valuations.1,
            1e-7
        );
        assert_float_absolute_eq!(
            indicator.pnl_weekly.value,
            indicator.pnl_daily.value + previous_valuations.2,
            1e-7
        );
        assert_float_absolute_eq!(
            indicator.pnl_monthly.value,
            indicator.pnl_daily.value + previous_valuations.3,
            1e-7
        );
    }

    #[test]
    fn compute_position_without_trade() {
        let instrument = make_instrument_("PAEEM");
        let position = Position {
            instrument,
            trades: Default::default(),
        };
        let date = chrono::NaiveDate::from_ymd_opt(2022, 3, 17).unwrap();
        let indicator = make_indicator_(&position, date, 21.92, &mut Default::default());
        check_indicator_(
            &indicator,
            0.0,
            0.0,
            0.0,
            (0.0, 0.0, 0.0),
            0.0,
            (0.0, 0.0, 0.0, 0.0),
        );
    }

    #[test]
    fn compute_position_with_trade_01() {
        let instrument = make_instrument_("PAEEM");
        let position = Position {
            instrument,
            trades: vec![Trade {
                date: chrono::DateTime::parse_from_rfc3339("2022-03-17T10:00:00-00:00")
                    .unwrap()
                    .naive_local(),
                way: Way::Buy,
                quantity: 14.0,
                price: 22.184,
                tax: 1.55,
            }],
        };

        let mut portfolio_indicators = Vec::new();
        let mut pnl_accumulator_by_instrument = Default::default();
        let date = chrono::NaiveDate::from_ymd_opt(2022, 3, 17).unwrap();
        for (pos, spot) in [
            21.92, 22.41, 22.41, 22.41, 22.03, 22.55, 22.55, 22.53, 22.32, 22.32, 22.32, 22.35,
            22.53,
        ]
        .iter()
        .enumerate()
        {
            let date = date
                .checked_add_days(chrono::naive::Days::new(pos as u64))
                .unwrap();
            let portfolio_indicator = make_default_portfolio_indicator_(make_indicator_(
                &position,
                date,
                *spot,
                &mut pnl_accumulator_by_instrument,
            ));
            portfolio_indicators.push(portfolio_indicator);
        }

        let indicator_17 = portfolio_indicators
            .first()
            .unwrap()
            .positions
            .first()
            .unwrap();
        let indicator_18 = portfolio_indicators
            .get(1)
            .unwrap()
            .positions
            .first()
            .unwrap();
        let indicator_20 = portfolio_indicators
            .get(3)
            .unwrap()
            .positions
            .first()
            .unwrap();
        let indicator_21 = portfolio_indicators
            .get(4)
            .unwrap()
            .positions
            .first()
            .unwrap();

        check_indicator_(
            indicator_17,
            indicator_17.spot.close() * 14.0,
            312.126,
            14.0 * 22.184,
            (14.0, 14.0, 0.0),
            312.126 / 14.0,
            (0.0, 0.0, 0.0, 0.0),
        );

        check_indicator_(
            indicator_18,
            indicator_18.spot.close() * 14.0,
            312.126,
            0.0,
            (14.0, 14.0, 0.0),
            312.126 / 14.0,
            (
                indicator_17.valuation,
                indicator_17.pnl_current.value,
                indicator_17.pnl_weekly.value,
                indicator_17.pnl_monthly.value,
            ),
        );

        check_indicator_(
            indicator_21,
            indicator_21.spot.close() * 14.0,
            312.126,
            0.0,
            (14.0, 14.0, 0.0),
            312.126 / 14.0,
            (
                indicator_20.valuation,
                indicator_20.pnl_current.value,
                0.0,
                indicator_20.pnl_monthly.value,
            ),
        );
    }
}
