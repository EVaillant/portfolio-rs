use super::PortfolioIndicator;
use crate::marketdata::Instrument;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub struct RegionIndicatorInstrument {
    pub instrument: Rc<Instrument>,
    pub valuation_percent: f64,
}

pub struct RegionIndicator {
    pub region_name: String,
    pub valuation_percent: f64,
    pub instruments: Vec<RegionIndicatorInstrument>,
}

impl RegionIndicator {
    pub fn from_portfolio(indicator: &PortfolioIndicator) -> Vec<Self> {
        let regions = indicator
            .positions
            .iter()
            .filter(|position| !position.is_close && position.instrument.region.is_some())
            .map(|position| position.instrument.region.as_ref().unwrap())
            .collect::<HashSet<_>>();

        let valuation = indicator
            .positions
            .iter()
            .filter(|position| !position.is_close)
            .map(|position| &position.valuation)
            .sum::<f64>();

        regions
            .into_iter()
            .map(|region| {
                let mut valuation_by_instrument: HashMap<Rc<Instrument>, f64> = Default::default();
                let mut valuation_by_region = 0.0;
                indicator
                    .positions
                    .iter()
                    .filter(|position| {
                        !position.is_close
                            && position
                                .instrument
                                .region
                                .as_ref()
                                .is_some_and(|item| *item == *region)
                    })
                    .for_each(|position| {
                        let value = valuation_by_instrument
                            .entry(position.instrument.clone())
                            .or_insert(0.0);
                        *value += position.valuation;
                        valuation_by_region += position.valuation;
                    });
                RegionIndicator {
                    region_name: region.to_string(),
                    valuation_percent: valuation_by_region / valuation,
                    instruments: valuation_by_instrument
                        .iter()
                        .map(|(key, value)| RegionIndicatorInstrument {
                            instrument: key.clone(),
                            valuation_percent: value / valuation_by_region,
                        })
                        .collect(),
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
        instruments: &[RegionIndicatorInstrument],
        instrument_name: &str,
        valuation_percent: f64,
    ) {
        let have_valuation_percent = instruments
            .iter()
            .find(|item| item.instrument.name == instrument_name)
            .map(|item| item.valuation_percent);
        assert!(have_valuation_percent.is_some());
        assert_float_absolute_eq!(have_valuation_percent.unwrap(), valuation_percent, 1e-7);
    }

    fn check_region_indicator_valuation_percent_(
        regions: &[RegionIndicator],
        region_name: &str,
        valuation_percent: f64,
    ) {
        let have_valuation_percent = regions
            .iter()
            .find(|item| item.region_name == region_name)
            .map(|item| item.valuation_percent);
        assert!(have_valuation_percent.is_some());
        assert_float_absolute_eq!(have_valuation_percent.unwrap(), valuation_percent, 1e-7);
    }

    #[test]
    fn region_indicator_01() {
        let result = RegionIndicator::from_portfolio(&make_portfolio_indicator_(vec![]));
        assert!(result.is_empty());
    }

    #[test]
    fn region_indicator_02() {
        let result = RegionIndicator::from_portfolio(&make_portfolio_indicator_(vec![
            make_position_indicator_("PAEEM", Some("Europe"), 500.0, false),
        ]));

        assert!(result.len() == 1);
        check_region_indicator_valuation_percent_(&result, "Europe", 1.0);

        assert!(result[0].instruments.len() == 1);
        check_instrument_valuation_percent_(&result[0].instruments, "PAEEM", 1.0);
    }

    #[test]
    fn region_indicator_03() {
        let result = RegionIndicator::from_portfolio(&make_portfolio_indicator_(vec![
            make_position_indicator_("PAEEM", Some("Europe"), 300.0, false),
            make_position_indicator_("CAC", Some("Europe"), 100.0, false),
            make_position_indicator_("ALV", Some("Europe"), 100.0, true),
        ]));
        assert!(result.len() == 1);
        check_region_indicator_valuation_percent_(&result, "Europe", 1.0);

        assert!(result[0].instruments.len() == 2);
        check_instrument_valuation_percent_(&result[0].instruments, "PAEEM", 0.75);
        check_instrument_valuation_percent_(&result[0].instruments, "CAC", 0.25);
    }

    #[test]
    fn region_indicator_04() {
        let result = RegionIndicator::from_portfolio(&make_portfolio_indicator_(vec![
            make_position_indicator_("PAEEM", Some("Europe"), 400.0, false),
            make_position_indicator_("CAC", Some("Europe"), 200.0, false),
            make_position_indicator_("ESE", Some("US"), 60.0, false),
            make_position_indicator_("RS2K1", Some("US"), 120.0, false),
            make_position_indicator_("RS2K2", Some("US"), 20.0, false),
        ]));
        assert!(result.len() == 2);
        check_region_indicator_valuation_percent_(&result, "Europe", 0.75);
        check_region_indicator_valuation_percent_(&result, "US", 0.25);

        let region = result.iter().find(|item| item.region_name == "Europe");
        assert!(region.is_some());
        let region = region.unwrap();
        assert!(region.instruments.len() == 2);
        check_instrument_valuation_percent_(&region.instruments, "PAEEM", 400.0 / 600.0);
        check_instrument_valuation_percent_(&region.instruments, "CAC", 200.0 / 600.0);

        let region = result.iter().find(|item| item.region_name == "US");
        assert!(region.is_some());
        let region = region.unwrap();
        assert!(region.instruments.len() == 3);
        check_instrument_valuation_percent_(&region.instruments, "ESE", 60.0 / 200.0);
        check_instrument_valuation_percent_(&region.instruments, "RS2K1", 120.0 / 200.0);
        check_instrument_valuation_percent_(&region.instruments, "RS2K2", 20.0 / 200.0);
    }
}
