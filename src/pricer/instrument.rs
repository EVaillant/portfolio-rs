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
