use super::primitive;
use crate::alias::{Date, Duration};
use crate::historical::DataFrame;
use crate::marketdata::Instrument;
use crate::portfolio::{Position, Way};
use crate::pricer::PortfolioIndicator;
use std::rc::Rc;

use log::debug;

#[derive(Default, Debug)]
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
    pub fees: f64,
    pub pnl_currency: f64,
    pub pnl_percent: f64,
    pub pnl_volatility_3m: f64,
    pub twr: f64,
    pub irr: Option<f64>,
    pub earning: f64,
    pub earning_latent: f64,
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

        let (quantity, quantity_buy, quantity_sell, unit_price, fees) =
            Self::compute_quantity_(position, date);

        let is_close = quantity.abs() < 1e-7;

        let valuation = spot.close * quantity;
        let nominal = unit_price * quantity;

        let cashflow = Self::compute_cashflow_(position, date);
        let (pnl_currency, pnl_percent) = primitive::pnl(valuation, nominal);

        let pnl_volatility_3m = primitive::volatility_from(
            date,
            Duration::days(90),
            previous_indicators,
            pnl_percent,
            |item| item.pnl_percent,
            |item| item.date,
        );

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
        let irr = Self::compute_irr_(
            date,
            valuation,
            cashflow,
            fees,
            dividends,
            previous_indicators,
        );

        let earning = dividends + Self::compute_earning_without_div_(position, date);
        let earning_latent = earning + valuation;

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
            fees,
            pnl_currency,
            pnl_percent,
            pnl_volatility_3m,
            twr,
            irr,
            earning,
            earning_latent,
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
                |(mut quantity, mut quantity_buy, mut quantity_sell, mut unit_price, mut fees),
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
                                (quantity * unit_price + trade.price * trade.quantity + trade.fees)
                                    / (quantity + trade.quantity);
                            quantity += trade.quantity;
                            quantity_buy += trade.quantity;
                        }
                    };
                    fees += trade.fees;
                    (quantity, quantity_buy, quantity_sell, unit_price, fees)
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
            .filter(|trade| trade.date.date() <= date)
            .map(|trade| match trade.way {
                Way::Sell => trade.price * trade.quantity - trade.fees,
                Way::Buy => -trade.price * trade.quantity - trade.fees,
            })
            .sum()
    }

    fn compute_irr_(
        date: Date,
        valuation: f64,
        cashflow: f64,
        fees: f64,
        dividends: f64,
        previous_positions: &[PositionIndicator],
    ) -> Option<f64> {
        let mut previous_flow = 0.0;
        let mut is_close = false;
        let mut stop = false;
        let mut cashflows = previous_positions
            .iter()
            .take_while(|position| {
                stop = is_close;
                is_close = position.is_close;
                !stop
            })
            .map(|position| {
                let global_flow = position.cashflow + position.fees - position.dividends;
                let flow = previous_flow - global_flow;
                previous_flow = global_flow;
                primitive::CashFlow {
                    date: position.date,
                    amount: flow,
                }
            })
            .filter(|cashflow| cashflow.amount.abs() > 1e-7)
            .collect::<Vec<_>>();
        if !stop {
            let global_flow = -valuation + cashflow + fees - dividends;
            let flow = previous_flow - global_flow;
            if flow.abs() > 1e-7 {
                cashflows.push(primitive::CashFlow { date, amount: flow });
            }
        }
        primitive::xirr(&cashflows, 0.5)
    }
}

pub struct ClosePositionIndicator {
    pub open: Date,
    pub close: Date,
    pub instrument: Rc<Instrument>,
    pub position_index: usize,
    pub pnl_currency: f64,
    pub fees: f64,
    pub dividends: f64,
    pub twr: f64,
    pub irr: Option<f64>,
}

impl ClosePositionIndicator {
    pub fn from_positions(positions: &[&PositionIndicator]) -> Self {
        let open_position = positions.first().unwrap();
        let close_position = positions.iter().find(|item| item.is_close).unwrap();

        Self {
            open: open_position.date,
            close: close_position.date,
            instrument: open_position.instrument.clone(),
            position_index: open_position.position_index,
            pnl_currency: close_position.earning,
            fees: close_position.fees,
            dividends: close_position.dividends,
            twr: close_position.twr,
            irr: close_position.irr,
        }
    }

    pub fn from_portfolios(portfolios: &[PortfolioIndicator]) -> Vec<ClosePositionIndicator> {
        if let Some(portfolio) = portfolios.last() {
            portfolio
                .positions
                .iter()
                .filter(|position| position.is_close)
                .map(|position| position.position_index)
                .map(|position_index| {
                    let positions = portfolios
                        .iter()
                        .flat_map(|item| {
                            item.positions
                                .iter()
                                .filter(|position| position.position_index == position_index)
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>();
                    ClosePositionIndicator::from_positions(&positions)
                })
                .collect::<Vec<_>>()
        } else {
            Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marketdata::{Currency, Dividend, Instrument, Market};
    use crate::portfolio::{Position, Trade, Way};
    use assert_float_eq::*;

    fn make_instrument_(name: &str, dividends: Option<Vec<Dividend>>) -> Rc<Instrument> {
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
            dividends,
        })
    }

    fn make_date_(year: i32, month: u32, day: u32) -> Date {
        chrono::NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn make_spot_(date: Date, value: f64) -> DataFrame {
        DataFrame::new(date, value, value, value, value)
    }

    fn make_position_() -> Position {
        let instrument = make_instrument_("PAEEM", None);
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
                    fees: 1.55,
                },
                Trade {
                    date: chrono::DateTime::parse_from_rfc3339("2022-03-19T10:00:00-00:00")
                        .unwrap()
                        .naive_local(),
                    way: Way::Buy,
                    quantity: 20.0,
                    price: 19.5,
                    fees: 1.0,
                },
                Trade {
                    date: chrono::DateTime::parse_from_rfc3339("2022-03-21T10:00:00-00:00")
                        .unwrap()
                        .naive_local(),
                    way: Way::Sell,
                    quantity: 10.0,
                    price: 20.0,
                    fees: 1.2,
                },
                Trade {
                    date: chrono::DateTime::parse_from_rfc3339("2022-03-22T10:00:00-00:00")
                        .unwrap()
                        .naive_local(),
                    way: Way::Sell,
                    quantity: 24.0,
                    price: 21.0,
                    fees: 1.3,
                },
            ],
        }
    }

    #[test]
    fn compute_position_without_trade() {
        let instrument = make_instrument_("PAEEM", None);
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
            let (quantity, quantity_buy, quantity_sell, unit_price, fees) =
                PositionIndicator::compute_quantity_(&position, make_date_(2022, 3, 17));
            assert_float_absolute_eq!(quantity, 14.0, 1e-7);
            assert_float_absolute_eq!(quantity_buy, 14.0, 1e-7);
            assert_float_absolute_eq!(quantity_sell, 0.0, 1e-7);
            assert_float_absolute_eq!(unit_price, 21.6107142, 1e-7);
            assert_float_absolute_eq!(fees, 1.55, 1e-7);
        }
        {
            let (quantity, quantity_buy, quantity_sell, unit_price, fees) =
                PositionIndicator::compute_quantity_(&position, make_date_(2022, 3, 19));
            assert_float_absolute_eq!(quantity, 34.0, 1e-7);
            assert_float_absolute_eq!(quantity_buy, 34.0, 1e-7);
            assert_float_absolute_eq!(quantity_sell, 0.0, 1e-7);
            assert_float_absolute_eq!(unit_price, 20.398529411764706, 1e-7);
            assert_float_absolute_eq!(fees, 2.55, 1e-7);
        }
        {
            let (quantity, quantity_buy, quantity_sell, unit_price, fees) =
                PositionIndicator::compute_quantity_(&position, make_date_(2022, 3, 20));
            assert_float_absolute_eq!(quantity, 34.0, 1e-7);
            assert_float_absolute_eq!(quantity_buy, 34.0, 1e-7);
            assert_float_absolute_eq!(quantity_sell, 0.0, 1e-7);
            assert_float_absolute_eq!(unit_price, 20.398529411764706, 1e-7);
            assert_float_absolute_eq!(fees, 2.55, 1e-7);
        }
        {
            let (quantity, quantity_buy, quantity_sell, unit_price, fees) =
                PositionIndicator::compute_quantity_(&position, make_date_(2022, 3, 21));
            assert_float_absolute_eq!(quantity, 24.0, 1e-7);
            assert_float_absolute_eq!(quantity_buy, 34.0, 1e-7);
            assert_float_absolute_eq!(quantity_sell, 10.0, 1e-7);
            assert_float_absolute_eq!(unit_price, 20.398529411764706, 1e-7);
            assert_float_absolute_eq!(fees, 3.75, 1e-7);
        }
        {
            let (quantity, quantity_buy, quantity_sell, unit_price, fees) =
                PositionIndicator::compute_quantity_(&position, make_date_(2022, 3, 22));
            assert_float_absolute_eq!(quantity, 0.0, 1e-7);
            assert_float_absolute_eq!(quantity_buy, 34.0, 1e-7);
            assert_float_absolute_eq!(quantity_sell, 34.0, 1e-7);
            assert_float_absolute_eq!(unit_price, 0.0, 1e-7);
            assert_float_absolute_eq!(fees, 5.05, 1e-7);
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
            assert_float_absolute_eq!(earning, 0.0 - 302.55, 1e-7);
        }
        {
            let earning =
                PositionIndicator::compute_earning_without_div_(&position, make_date_(2022, 3, 19));
            assert_float_absolute_eq!(earning, 0.0 - 693.55, 1e-7);
        }
        {
            let earning =
                PositionIndicator::compute_earning_without_div_(&position, make_date_(2022, 3, 20));
            assert_float_absolute_eq!(earning, 0.0 - 693.55, 1e-7);
        }
        {
            let earning =
                PositionIndicator::compute_earning_without_div_(&position, make_date_(2022, 3, 21));
            assert_float_absolute_eq!(earning, 198.8 - 693.55, 1e-7);
        }
        {
            let earning =
                PositionIndicator::compute_earning_without_div_(&position, make_date_(2022, 3, 22));
            assert_float_absolute_eq!(earning, 701.5 - 693.55, 1e-7);
        }
    }

    #[test]
    fn compute_dividends() {
        {
            let instrument = make_instrument_("WHATEVER", Some(vec![]));
            let position = Position {
                instrument,
                trades: Default::default(),
            };
            let result = PositionIndicator::compute_dividends_(&position, make_date_(2022, 3, 20));
            assert_float_absolute_eq!(result, 0.0, 1e-7);
        }

        {
            let instrument = make_instrument_(
                "WHATEVER",
                Some(vec![Dividend {
                    record_date: chrono::DateTime::parse_from_rfc3339("2022-03-01T08:00:00-00:00")
                        .unwrap()
                        .naive_local(),
                    payment_date: chrono::DateTime::parse_from_rfc3339("2022-03-10T08:00:00-00:00")
                        .unwrap()
                        .naive_local(),
                    value: 5.0,
                }]),
            );
            let position = Position {
                instrument,
                trades: Default::default(),
            };
            let result = PositionIndicator::compute_dividends_(&position, make_date_(2022, 3, 20));
            assert_float_absolute_eq!(result, 0.0, 1e-7);
        }

        {
            let instrument = make_instrument_(
                "WHATEVER",
                Some(vec![Dividend {
                    record_date: chrono::DateTime::parse_from_rfc3339("2022-03-01T08:00:00-00:00")
                        .unwrap()
                        .naive_local(),
                    payment_date: chrono::DateTime::parse_from_rfc3339("2022-03-10T08:00:00-00:00")
                        .unwrap()
                        .naive_local(),
                    value: 5.0,
                }]),
            );
            let position = Position {
                instrument,
                trades: vec![
                    Trade {
                        date: chrono::DateTime::parse_from_rfc3339("2022-02-17T10:00:00-00:00")
                            .unwrap()
                            .naive_local(),
                        way: Way::Buy,
                        quantity: 14.0,
                        price: 21.5,
                        fees: 1.55,
                    },
                    Trade {
                        date: chrono::DateTime::parse_from_rfc3339("2022-03-09T10:00:00-00:00")
                            .unwrap()
                            .naive_local(),
                        way: Way::Buy,
                        quantity: 20.0,
                        price: 19.5,
                        fees: 1.0,
                    },
                    Trade {
                        date: chrono::DateTime::parse_from_rfc3339("2022-03-21T10:00:00-00:00")
                            .unwrap()
                            .naive_local(),
                        way: Way::Buy,
                        quantity: 10.0,
                        price: 20.0,
                        fees: 1.2,
                    },
                ],
            };
            let result = PositionIndicator::compute_dividends_(&position, make_date_(2022, 3, 20));
            assert_float_absolute_eq!(result, 14.0 * 5.0, 1e-7);

            let result = PositionIndicator::compute_dividends_(&position, make_date_(2022, 3, 5));
            assert_float_absolute_eq!(result, 0.0, 1e-7);

            let result = PositionIndicator::compute_dividends_(&position, make_date_(2022, 2, 20));
            assert_float_absolute_eq!(result, 0.0, 1e-7);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_position_indicator_(
        instrument_name: &str,
        position_index: usize,
        date: Date,
        is_close: bool,
        earning: f64,
        fees: f64,
        dividends: f64,
        cashflow: f64,
    ) -> PositionIndicator {
        let instrument = make_instrument_(instrument_name, None);
        PositionIndicator {
            date,
            instrument,
            position_index,
            is_close,
            earning,
            fees,
            dividends,
            cashflow,
            ..Default::default()
        }
    }

    #[test]
    fn close_position() {
        let date1 = make_date_(2025, 1, 1);
        let date2 = make_date_(2025, 1, 2);
        let date3 = make_date_(2025, 1, 3);
        let date4 = make_date_(2025, 1, 4);
        {
            let result = ClosePositionIndicator::from_positions(&[
                &build_position_indicator_("ESE", 1, date1, false, 10.0, 2.0, 0.0, 0.0),
                &build_position_indicator_("ESE", 1, date2, false, 20.0, 3.0, 5.0, 0.0),
                &build_position_indicator_("ESE", 1, date3, false, 50.0, 4.0, 10.0, 0.0),
                &build_position_indicator_("ESE", 1, date4, true, 100.0, 5.0, 12.0, 0.0),
            ]);
            assert!(result.open == date1);
            assert!(result.close == date4);
            assert!(result.instrument.name == "ESE");
            assert!(result.position_index == 1);
            assert_float_absolute_eq!(result.pnl_currency, 100.0, 1e-7);
            assert_float_absolute_eq!(result.dividends, 12.0, 1e-7);
            assert_float_absolute_eq!(result.fees, 5.0, 1e-7);
        }
        {
            let portfolios = vec![];
            let results = ClosePositionIndicator::from_portfolios(&portfolios);
            assert!(results.is_empty());
        }
        {
            let portfolios = vec![
                PortfolioIndicator {
                    date: date1,
                    positions: vec![
                        build_position_indicator_("ESE", 1, date1, false, 10.0, 2.0, 0.0, 0.0),
                        build_position_indicator_("ASA", 2, date1, false, 10.0, 2.0, 0.0, 0.0),
                        build_position_indicator_("BSB", 3, date1, false, 10.0, 2.0, 0.0, 0.0),
                        build_position_indicator_("CSC", 4, date1, false, 10.0, 2.0, 0.0, 0.0),
                    ],
                    ..Default::default()
                },
                PortfolioIndicator {
                    date: date2,
                    positions: vec![
                        build_position_indicator_("ESE", 1, date2, false, 10.0, 2.0, 0.0, 0.0),
                        build_position_indicator_("ASA", 2, date2, false, 10.0, 2.0, 0.0, 0.0),
                        build_position_indicator_("BSB", 3, date2, true, 8.0, 9.0, 10.0, 0.0),
                        build_position_indicator_("CSC", 4, date2, false, 10.0, 2.0, 0.0, 0.0),
                    ],
                    ..Default::default()
                },
                PortfolioIndicator {
                    date: date3,
                    positions: vec![
                        build_position_indicator_("ESE", 1, date3, false, 10.0, 2.0, 0.0, 0.0),
                        build_position_indicator_("ASA", 2, date3, true, 5.0, 4.0, 9.0, 0.0),
                        build_position_indicator_("BSB", 3, date3, true, 10.0, 2.0, 0.0, 0.0),
                        build_position_indicator_("CSC", 4, date3, false, 10.0, 2.0, 0.0, 0.0),
                    ],
                    ..Default::default()
                },
                PortfolioIndicator {
                    date: date4,
                    positions: vec![
                        build_position_indicator_("ESE", 1, date4, true, 1.0, 0.0, 2.0, 0.0),
                        build_position_indicator_("ASA", 2, date4, true, 10.0, 2.0, 0.0, 0.0),
                        build_position_indicator_("BSB", 3, date4, true, 10.0, 2.0, 0.0, 0.0),
                        build_position_indicator_("CSC", 4, date4, false, 10.0, 2.0, 0.0, 0.0),
                    ],
                    ..Default::default()
                },
            ];
            let results = ClosePositionIndicator::from_portfolios(&portfolios);
            assert!(results.len() == 3);

            let result = results.iter().find(|item| item.instrument.name == "ESE");
            assert!(result.is_some());
            let result = result.unwrap();
            assert!(result.open == date1);
            assert!(result.close == date4);
            assert!(result.instrument.name == "ESE");
            assert!(result.position_index == 1);
            assert_float_absolute_eq!(result.pnl_currency, 1.0, 1e-7);
            assert_float_absolute_eq!(result.fees, 0.0, 1e-7);
            assert_float_absolute_eq!(result.dividends, 2.0, 1e-7);

            let result = results.iter().find(|item| item.instrument.name == "ASA");
            assert!(result.is_some());
            let result = result.unwrap();
            assert!(result.open == date1);
            assert!(result.close == date3);
            assert!(result.instrument.name == "ASA");
            assert!(result.position_index == 2);
            assert_float_absolute_eq!(result.pnl_currency, 5.0, 1e-7);
            assert_float_absolute_eq!(result.fees, 4.0, 1e-7);
            assert_float_absolute_eq!(result.dividends, 9.0, 1e-7);

            let result = results.iter().find(|item| item.instrument.name == "BSB");
            assert!(result.is_some());
            let result = result.unwrap();
            assert!(result.open == date1);
            assert!(result.close == date2);
            assert!(result.instrument.name == "BSB");
            assert!(result.position_index == 3);
            assert_float_absolute_eq!(result.pnl_currency, 8.0, 1e-7);
            assert_float_absolute_eq!(result.fees, 9.0, 1e-7);
            assert_float_absolute_eq!(result.dividends, 10.0, 1e-7);
        }
    }

    /*#[test]
    fn compute_irr() {
        let date1 = make_date_(2025, 1, 1);
        let date2 = make_date_(2025, 2, 1);
        let date3 = make_date_(2025, 3, 1);
        let date4 = make_date_(2025, 4, 1);
        let date5 = make_date_(2025, 5, 1);

        let result = PositionIndicator::compute_irr_(
            date5,
            0.0,
            -30.0,
            4.0,
            0.0,
            &[
                build_position_indicator_("ESE", 1, date1, false, 0.0, 0.0, 2.0, 400.0),
                build_position_indicator_("ESE", 1, date2, false, 0.0, 0.0, 2.0, 400.0),
                build_position_indicator_("ESE", 1, date3, true, 0.0, 0.0, 4.0, -30.0),
                build_position_indicator_("ESE", 1, date4, true, 0.0, 0.0, 4.0, -30.0),
            ],
        );
        assert!(result.is_some());
        assert_float_absolute_eq!(result.unwrap(), 0.6605098738699448, 1e-7);

        let result = PositionIndicator::compute_irr_(
            date3,
            500.0,
            400.0,
            0.0,
            0.0,
            &[
                build_position_indicator_("ESE", 1, date1, false, 0.0, 0.0, 0.0, 400.0),
                build_position_indicator_("ESE", 1, date2, false, 0.0, 0.0, 0.0, 400.0),
            ],
        );
        assert!(result.is_some());
        assert_float_absolute_eq!(result.unwrap(), 2.9767477733376104, 1e-7);
    }*/

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
