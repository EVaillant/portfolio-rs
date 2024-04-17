use super::primitive;
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
    pub position_index: usize,
    pub quantity: f64,
    pub quantity_buy: f64,
    pub quantity_sell: f64,
    pub unit_price: f64,
    pub valuation: f64,
    pub nominal: f64,
    pub cashflow: f64,
    pub dividends: f64,
    pub tax: f64,
    pub pnl_currency: f64,
    pub pnl_percent: f64,
    pub twr: f64,
    pub earning: f64,
    pub earning_latent: f64,
    pub cost: f64,
    pub is_close: bool,
}

impl PositionIndicator {
    pub fn from_position(
        position: &Position,
        date: Date,
        position_index: usize,
        spot: &DataFrame,
        previous_indicators: &[PositionIndicator],
    ) -> PositionIndicator {
        debug!(
            "price position {} at {} with spot:{}",
            position.instrument.name, date, spot.close
        );

        let (quantity, quantity_buy, quantity_sell, unit_price, tax) =
            Self::compute_quantity_(position, date);

        let is_close = quantity.abs() < 1e-7;

        let valuation = spot.close * quantity;
        let nominal = unit_price * quantity;

        let cashflow = Self::compute_cashflow_(position, date);
        let (pnl_currency, pnl_percent) = primitive::pnl(valuation, nominal);

        let (previous_twr, begin_valuation, delta_cashflow) =
            if let Some(previous_indicator) = previous_indicators.last() {
                (
                    previous_indicator.twr,
                    previous_indicator.valuation,
                    cashflow - previous_indicator.cashflow,
                )
            } else {
                (0.0, nominal, 0.0)
            };

        let twr = primitive::twr(begin_valuation, valuation, delta_cashflow, previous_twr);

        let dividends = Self::compute_dividends_(position, date);

        let earning = dividends + Self::compute_earning_without_div_(position, date);
        let earning_latent = earning + valuation;

        let cost = Self::compute_cost_(position, date);

        PositionIndicator {
            date,
            spot: *spot,
            instrument: position.instrument.clone(),
            position_index,
            quantity,
            quantity_buy,
            quantity_sell,
            unit_price,
            valuation,
            nominal,
            cashflow,
            dividends,
            tax,
            pnl_currency,
            pnl_percent,
            twr,
            earning,
            earning_latent,
            cost,
            is_close,
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

    fn compute_cashflow_(position: &Position, date: Date) -> f64 {
        position
            .trades
            .iter()
            .filter(|trade| trade.date.date() <= date)
            .map(|trade| match trade.way {
                Way::Sell => -1.0,
                Way::Buy => 1.0,
            } * trade.quantity * trade.price)
            .sum()
    }

    fn compute_dividends_(position: &Position, date: Date) -> f64 {
        position
            .instrument
            .dividends
            .as_ref()
            .map_or(0.0, |dividends| {
                dividends
                    .iter()
                    .filter(|dividend| dividend.payment_date.date() <= date)
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
    }

    fn compute_earning_without_div_(position: &Position, date: Date) -> f64 {
        position
            .trades
            .iter()
            .filter(|trade| trade.date.date() <= date && trade.way == Way::Sell)
            .map(|trade| trade.price * trade.quantity - trade.tax)
            .sum()
    }

    fn compute_cost_(position: &Position, date: Date) -> f64 {
        position
            .trades
            .iter()
            .filter(|trade| trade.date.date() <= date && trade.way == Way::Buy)
            .map(|trade| trade.price * trade.quantity + trade.tax)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marketdata::{Currency, Instrument, Market};
    use crate::portfolio::{Position, Trade, Way};
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
            region: None,
            fund_category: String::from("category"),
            dividends: None,
        })
    }

    fn make_date_(year: i32, month: u32, day: u32) -> Date {
        chrono::NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn make_spot_(date: Date, value: f64) -> DataFrame {
        DataFrame::new(date, value, value, value, value)
    }

    fn make_position_() -> Position {
        let instrument = make_instrument_("PAEEM");
        Position {
            instrument,
            trades: vec![
                Trade {
                    date: chrono::DateTime::parse_from_rfc3339("2022-03-17T10:00:00-00:00")
                        .unwrap()
                        .naive_local(),
                    way: Way::Buy,
                    quantity: 14.0,
                    price: 21.5,
                    tax: 1.55,
                },
                Trade {
                    date: chrono::DateTime::parse_from_rfc3339("2022-03-19T10:00:00-00:00")
                        .unwrap()
                        .naive_local(),
                    way: Way::Buy,
                    quantity: 20.0,
                    price: 19.5,
                    tax: 1.0,
                },
                Trade {
                    date: chrono::DateTime::parse_from_rfc3339("2022-03-21T10:00:00-00:00")
                        .unwrap()
                        .naive_local(),
                    way: Way::Sell,
                    quantity: 10.0,
                    price: 20.0,
                    tax: 1.2,
                },
                Trade {
                    date: chrono::DateTime::parse_from_rfc3339("2022-03-22T10:00:00-00:00")
                        .unwrap()
                        .naive_local(),
                    way: Way::Sell,
                    quantity: 24.0,
                    price: 21.0,
                    tax: 1.3,
                },
            ],
        }
    }

    #[test]
    fn compute_position_without_trade() {
        let instrument = make_instrument_("PAEEM");
        let position = Position {
            instrument,
            trades: Default::default(),
        };
        let date = make_date_(2022, 3, 17);
        let indicator = PositionIndicator::from_position(
            &position,
            date,
            0,
            &make_spot_(date, 21.92),
            Default::default(),
        );
        check_indicator_(&indicator, 0.0, 0.0, (0.0, 0.0), 0.0, true);
    }

    #[test]
    fn compute_position_with_trade() {
        let position = make_position_();
        let mut previous_indicators = Vec::new();
        {
            let date = make_date_(2022, 3, 17);
            let indicator = PositionIndicator::from_position(
                &position,
                date,
                0,
                &make_spot_(date, 21.0),
                &previous_indicators,
            );
            check_indicator_(
                &indicator,
                294.0,
                302.55,
                (-8.55, -0.028259792),
                -0.028259792,
                false,
            );
            previous_indicators.push(indicator);
        }
        {
            let date = make_date_(2022, 3, 19);
            let indicator = PositionIndicator::from_position(
                &position,
                date,
                0,
                &make_spot_(date, 22.0),
                &previous_indicators,
            );
            check_indicator_(
                &indicator,
                748.0,
                693.55,
                (54.45, 0.07850911974623322),
                0.18327549165427204,
                false,
            );
            previous_indicators.push(indicator);
        }
        {
            let date = make_date_(2022, 3, 20);
            let indicator = PositionIndicator::from_position(
                &position,
                date,
                0,
                &make_spot_(date, 21.5),
                &previous_indicators,
            );
            check_indicator_(
                &indicator,
                731.0,
                693.55,
                (37.45, 0.053997548842909734),
                0.15638286684394775,
                false,
            );
            previous_indicators.push(indicator);
        }
        {
            let date = make_date_(2022, 3, 21);
            let indicator = PositionIndicator::from_position(
                &position,
                date,
                0,
                &make_spot_(date, 21.75),
                &previous_indicators,
            );
            check_indicator_(
                &indicator,
                522.0,
                489.56470588235294,
                (32.43529411764706, 0.0662533342945714),
                0.1421455948855408,
                false,
            );
            previous_indicators.push(indicator);
        }
        {
            let date = make_date_(2022, 3, 22);
            let indicator = PositionIndicator::from_position(
                &position,
                date,
                0,
                &make_spot_(date, 22.5),
                &previous_indicators,
            );
            check_indicator_(&indicator, 0.0, 0.0, (0.0, 0.0), 0.1027612640274187, true);
            previous_indicators.push(indicator);
        }
    }

    #[test]
    fn compute_quantity() {
        let position = make_position_();
        {
            let (quantity, quantity_buy, quantity_sell, unit_price, tax) =
                PositionIndicator::compute_quantity_(&position, make_date_(2022, 3, 17));
            assert_float_absolute_eq!(quantity, 14.0, 1e-7);
            assert_float_absolute_eq!(quantity_buy, 14.0, 1e-7);
            assert_float_absolute_eq!(quantity_sell, 0.0, 1e-7);
            assert_float_absolute_eq!(unit_price, 21.6107142, 1e-7);
            assert_float_absolute_eq!(tax, 1.55, 1e-7);
        }
        {
            let (quantity, quantity_buy, quantity_sell, unit_price, tax) =
                PositionIndicator::compute_quantity_(&position, make_date_(2022, 3, 19));
            assert_float_absolute_eq!(quantity, 34.0, 1e-7);
            assert_float_absolute_eq!(quantity_buy, 34.0, 1e-7);
            assert_float_absolute_eq!(quantity_sell, 0.0, 1e-7);
            assert_float_absolute_eq!(unit_price, 20.398529411764706, 1e-7);
            assert_float_absolute_eq!(tax, 2.55, 1e-7);
        }
        {
            let (quantity, quantity_buy, quantity_sell, unit_price, tax) =
                PositionIndicator::compute_quantity_(&position, make_date_(2022, 3, 20));
            assert_float_absolute_eq!(quantity, 34.0, 1e-7);
            assert_float_absolute_eq!(quantity_buy, 34.0, 1e-7);
            assert_float_absolute_eq!(quantity_sell, 0.0, 1e-7);
            assert_float_absolute_eq!(unit_price, 20.398529411764706, 1e-7);
            assert_float_absolute_eq!(tax, 2.55, 1e-7);
        }
        {
            let (quantity, quantity_buy, quantity_sell, unit_price, tax) =
                PositionIndicator::compute_quantity_(&position, make_date_(2022, 3, 21));
            assert_float_absolute_eq!(quantity, 24.0, 1e-7);
            assert_float_absolute_eq!(quantity_buy, 34.0, 1e-7);
            assert_float_absolute_eq!(quantity_sell, 10.0, 1e-7);
            assert_float_absolute_eq!(unit_price, 20.398529411764706, 1e-7);
            assert_float_absolute_eq!(tax, 3.75, 1e-7);
        }
        {
            let (quantity, quantity_buy, quantity_sell, unit_price, tax) =
                PositionIndicator::compute_quantity_(&position, make_date_(2022, 3, 22));
            assert_float_absolute_eq!(quantity, 0.0, 1e-7);
            assert_float_absolute_eq!(quantity_buy, 34.0, 1e-7);
            assert_float_absolute_eq!(quantity_sell, 34.0, 1e-7);
            assert_float_absolute_eq!(unit_price, 0.0, 1e-7);
            assert_float_absolute_eq!(tax, 5.05, 1e-7);
        }
    }

    #[test]
    fn compute_cashflow() {
        let position = make_position_();
        {
            let cashflow = PositionIndicator::compute_cashflow_(&position, make_date_(2022, 3, 17));
            assert_float_absolute_eq!(cashflow, 301.0, 1e-7);
        }
        {
            let cashflow = PositionIndicator::compute_cashflow_(&position, make_date_(2022, 3, 19));
            assert_float_absolute_eq!(cashflow, 691.0, 1e-7);
        }
        {
            let cashflow = PositionIndicator::compute_cashflow_(&position, make_date_(2022, 3, 20));
            assert_float_absolute_eq!(cashflow, 691.0, 1e-7);
        }
        {
            let cashflow = PositionIndicator::compute_cashflow_(&position, make_date_(2022, 3, 21));
            assert_float_absolute_eq!(cashflow, 491.0, 1e-7);
        }
        {
            let cashflow = PositionIndicator::compute_cashflow_(&position, make_date_(2022, 3, 22));
            assert_float_absolute_eq!(cashflow, -13.0, 1e-7);
        }
    }

    #[test]
    fn compute_earning() {
        let position = make_position_();
        {
            let earning: f64 =
                PositionIndicator::compute_earning_without_div_(&position, make_date_(2022, 3, 17));
            assert_float_absolute_eq!(earning, 0.0, 1e-7);
        }
        {
            let earning =
                PositionIndicator::compute_earning_without_div_(&position, make_date_(2022, 3, 19));
            assert_float_absolute_eq!(earning, 0.0, 1e-7);
        }
        {
            let earning =
                PositionIndicator::compute_earning_without_div_(&position, make_date_(2022, 3, 20));
            assert_float_absolute_eq!(earning, 0.0, 1e-7);
        }
        {
            let earning =
                PositionIndicator::compute_earning_without_div_(&position, make_date_(2022, 3, 21));
            assert_float_absolute_eq!(earning, 198.8, 1e-7);
        }
        {
            let earning =
                PositionIndicator::compute_earning_without_div_(&position, make_date_(2022, 3, 22));
            assert_float_absolute_eq!(earning, 701.5, 1e-7);
        }
    }

    #[test]
    fn compute_cost() {
        let position = make_position_();
        {
            let cost: f64 = PositionIndicator::compute_cost_(&position, make_date_(2022, 3, 17));
            assert_float_absolute_eq!(cost, 302.55, 1e-7);
        }
        {
            let cost = PositionIndicator::compute_cost_(&position, make_date_(2022, 3, 19));
            assert_float_absolute_eq!(cost, 693.55, 1e-7);
        }
        {
            let cost = PositionIndicator::compute_cost_(&position, make_date_(2022, 3, 20));
            assert_float_absolute_eq!(cost, 693.55, 1e-7);
        }
        {
            let cost = PositionIndicator::compute_cost_(&position, make_date_(2022, 3, 21));
            assert_float_absolute_eq!(cost, 693.55, 1e-7);
        }
        {
            let cost = PositionIndicator::compute_cost_(&position, make_date_(2022, 3, 22));
            assert_float_absolute_eq!(cost, 693.55, 1e-7);
        }
    }

    fn check_indicator_(
        indicator: &PositionIndicator,
        valuation: f64,
        nominal: f64,
        pnl: (f64, f64),
        twr: f64,
        is_close: bool,
    ) {
        assert_float_absolute_eq!(indicator.valuation, valuation, 1e-7);
        assert_float_absolute_eq!(indicator.nominal, nominal, 1e-7);
        assert_float_absolute_eq!(indicator.pnl_currency, pnl.0, 1e-7);
        assert_float_absolute_eq!(indicator.pnl_percent, pnl.1, 1e-7);
        assert_float_absolute_eq!(indicator.twr, twr, 1e-7);
        assert_eq!(indicator.is_close, is_close);
    }
}
