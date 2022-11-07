use crate::marketdata::{Currency, Instrument, Market};
use std::rc::Rc;

pub struct Cache {
    currencies: Vec<Rc<Currency>>,
    markets: Vec<Rc<Market>>,
    instruments: Vec<Rc<Instrument>>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            currencies: Vec::new(),
            markets: Vec::new(),
            instruments: Vec::new(),
        }
    }

    pub fn add_currency(&mut self, currency: Currency) -> Rc<Currency> {
        self.currencies.push(Rc::new(currency));
        self.currencies.last().unwrap().clone()
    }

    pub fn get_currency_by<P>(&self, predicate: P) -> Option<Rc<Currency>>
    where
        P: Fn(&Currency) -> bool,
    {
        self.currencies
            .iter()
            .find(|currency| predicate(currency))
            .cloned()
    }

    pub fn add_market(&mut self, market: Market) -> Rc<Market> {
        self.markets.push(Rc::new(market));
        self.markets.last().unwrap().clone()
    }

    pub fn get_market_by<P>(&self, predicate: P) -> Option<Rc<Market>>
    where
        P: Fn(&Market) -> bool,
    {
        self.markets
            .iter()
            .find(|market| predicate(market))
            .cloned()
    }

    pub fn add_instrument(&mut self, instrument: Instrument) -> Rc<Instrument> {
        self.instruments.push(Rc::new(instrument));
        self.instruments.last().unwrap().clone()
    }

    pub fn get_instrument_by<P>(&self, predicate: P) -> Option<Rc<Instrument>>
    where
        P: Fn(&Instrument) -> bool,
    {
        self.instruments
            .iter()
            .find(|instrument| predicate(instrument))
            .cloned()
    }
}

impl Default for Cache {
    fn default() -> Self {
        Cache::new()
    }
}
