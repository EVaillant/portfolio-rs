use super::PortfolioIndicator;
use crate::marketdata::Instrument;
use std::collections::HashSet;
use std::rc::Rc;

pub struct InstrumentIndicator {
    pub instrument: Rc<Instrument>,
    pub valuation_percent: f64,
}

impl InstrumentIndicator {
    pub fn from_portfolio(indicator: &PortfolioIndicator) -> Vec<Self> {
        let instruments = indicator
            .positions
            .iter()
            .filter(|position| !position.is_close)
            .map(|position| position.instrument.clone())
            .collect::<HashSet<_>>();

        let valuation = indicator
            .positions
            .iter()
            .filter(|position| !position.is_close)
            .map(|position| &position.valuation)
            .sum::<f64>();

        instruments
            .into_iter()
            .map(|instrument| {
                let valuation_by_instrument = indicator
                    .positions
                    .iter()
                    .filter(|position| !position.is_close && position.instrument == instrument)
                    .map(|position| &position.valuation)
                    .sum::<f64>();
                InstrumentIndicator {
                    instrument: instrument.clone(),
                    valuation_percent: valuation_by_instrument / valuation,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marketdata::{Currency, Instrument, Market};
    use crate::pricer::PositionIndicator;
    use assert_float_eq::*;

    fn make_instrument_<T: Into<String>, U: Into<String>>(
        name: T,
        region: Option<U>,
    ) -> Rc<Instrument> {
        let currency = Rc::new(Currency {
            name: String::from("EUR"),
            parent_currency: None,
        });

        let market = Rc::new(Market {
            name: String::from("EPA"),
            description: String::from("EPA"),
        });

        Rc::new(Instrument {
            name: name.into(),
            isin: String::from("ISIN"),
            description: String::from("description"),
            market,
            currency,
            ticker_yahoo: None,
            region: region.map(|item| item.into()),
            fund_category: String::from("category"),
            dividends: None,
        })
    }

    fn make_position_indicator_<T: Into<String>, U: Into<String>>(
        instrument_name: T,
        instrument_region: Option<U>,
        valuation: f64,
        is_close: bool,
    ) -> PositionIndicator {
        let instrument = make_instrument_(instrument_name, instrument_region);
        PositionIndicator {
            instrument,
            valuation,
            is_close,
            ..Default::default()
        }
    }

    fn make_portfolio_indicator_(positions: Vec<PositionIndicator>) -> PortfolioIndicator {
        PortfolioIndicator {
            positions,
            ..Default::default()
        }
    }

    fn check_instrument_valuation_percent_(
        result: &[InstrumentIndicator],
        instrument_name: &str,
        valuation: f64,
    ) {
        let instrument_valuation_percent = result
            .iter()
            .find(|item| item.instrument.name == instrument_name)
            .map(|item| item.valuation_percent);
        assert!(instrument_valuation_percent.is_some());
        assert_float_absolute_eq!(instrument_valuation_percent.unwrap(), valuation, 1e-7);
    }

    #[test]
    fn instrument_indicator_01() {
        let result = InstrumentIndicator::from_portfolio(&make_portfolio_indicator_(vec![]));
        assert!(result.is_empty());
    }

    #[test]
    fn instrument_indicator_02() {
        let result = InstrumentIndicator::from_portfolio(&make_portfolio_indicator_(vec![
            make_position_indicator_("PAEEM", Some("Europe"), 300.0, false),
            make_position_indicator_("CAC", Some("Europe"), 100.0, false),
            make_position_indicator_("ALV", Some("Europe"), 100.0, true),
        ]));
        assert!(result.len() == 2);
        check_instrument_valuation_percent_(&result, "PAEEM", 300.0 / 400.0);
        check_instrument_valuation_percent_(&result, "CAC", 100.0 / 400.0);
    }
}
