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
            .filter(|position| !position.is_close)
            .map(|position| &position.instrument.region)
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
                    .filter(|position| !position.is_close && position.instrument.region == *region)
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
