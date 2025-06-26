mod cache;
mod serialize;

use crate::error::Error;
use crate::marketdata::{Currency, Instrument, Market};
use crate::portfolio::Portfolio;

use cache::*;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::rc::Rc;

impl serialize::Resolver for Referential {
    fn resolv_currency(&mut self, name: &str) -> Result<Rc<Currency>, Error> {
        self.get_currency_by_name(name).map_err(|err| {
            Error::new_referential(format!("unable to resolv {name} because {err:?}"))
        })
    }

    fn resolv_market(&mut self, name: &str) -> Result<Rc<Market>, Error> {
        self.get_market_by_name(name).map_err(|err| {
            Error::new_referential(format!("unable to resolv {name} because {err:?}"))
        })
    }

    fn resolv_instrument(&mut self, name: &str) -> Result<Rc<Instrument>, Error> {
        self.get_instrument_by_name(name).map_err(|err| {
            Error::new_referential(format!("unable to resolv {name} because {err:?}"))
        })
    }
}

pub struct Referential {
    marketdata_dir: String,
    cache: Cache,
}

impl Referential {
    pub fn new(path: &str) -> Self {
        Self {
            marketdata_dir: path.into(),
            cache: Default::default(),
        }
    }

    pub fn get_market_by_name(&mut self, name: &str) -> Result<Rc<Market>, Error> {
        let result = self.cache.get_market_by(|market| market.name == name);
        match result {
            Some(value) => Ok(value),
            None => {
                let filename = self.build_marketdata_filename_("market", name)?;
                let file = File::open(filename)?;
                let reader = BufReader::new(file);
                let market = serialize::from_reader(reader, self)?;
                Ok(self.cache.add_market(market))
            }
        }
    }

    pub fn get_currency_by_name(&mut self, name: &str) -> Result<Rc<Currency>, Error> {
        let result = self.cache.get_currency_by(|currency| currency.name == name);
        match result {
            Some(value) => Ok(value),
            None => {
                let filename = self.build_marketdata_filename_("currency", name)?;
                let file = File::open(filename)?;
                let reader = BufReader::new(file);
                let currency = serialize::from_reader(reader, self)?;
                Ok(self.cache.add_currency(currency))
            }
        }
    }

    pub fn get_instrument_by_name(&mut self, name: &str) -> Result<Rc<Instrument>, Error> {
        let result = self
            .cache
            .get_instrument_by(|instrument| instrument.name == name);
        match result {
            Some(value) => Ok(value),
            None => {
                let filename = self.build_marketdata_filename_("instrument", name)?;
                let file = File::open(filename)?;
                let reader = BufReader::new(file);
                let instrument = serialize::from_reader(reader, self)?;
                Ok(self.cache.add_instrument(instrument))
            }
        }
    }

    pub fn load_portfolio(&mut self, filename: &str) -> Result<Portfolio, Error> {
        let file = File::open(filename)?;
        let reader = BufReader::new(file);
        serialize::from_reader(reader, self)
    }

    fn build_marketdata_filename_(&self, kind: &str, name: &str) -> Result<PathBuf, Error> {
        let mut filename = PathBuf::new();
        filename.push(&self.marketdata_dir);
        filename.push(kind);
        filename.push(name);
        filename.set_extension("json");
        if !filename.is_file() {
            return Err(Error::new_referential(format!(
                "{} is not valid file",
                filename.display()
            )));
        }
        Ok(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_by_name() {
        let mut referential = Referential::new(concat!(env!("CARGO_MANIFEST_DIR"), "/data"));
        let result = referential.get_currency_by_name("EUR");
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.name == "EUR");
        assert!(result.parent_currency.is_none());

        let result = referential.get_currency_by_name("XXX");
        assert!(result.is_err());

        let result = referential.get_currency_by_name("EUR");
        assert!(result.is_ok());
    }

    #[test]
    fn market_by_name() {
        let mut referential = Referential::new(concat!(env!("CARGO_MANIFEST_DIR"), "/data"));
        let result = referential.get_market_by_name("EPA");
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.name == "EPA");
        assert!(result.description == "Euronext Paris");

        let result = referential.get_market_by_name("XXX");
        assert!(result.is_err());

        let result = referential.get_market_by_name("EPA");
        assert!(result.is_ok());
    }

    #[test]
    fn instrument_by_name() {
        let mut referential = Referential::new(concat!(env!("CARGO_MANIFEST_DIR"), "/data"));
        let result = referential.get_instrument_by_name("ESE");
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.name == "ESE");
        assert!(result.isin == "FR0011550185");
        assert!(result.description == "BNP Paribas Easy S&P 500 UCITS ETF EUR C");

        let result = referential.get_instrument_by_name("XXX");
        assert!(result.is_err());

        let result = referential.get_instrument_by_name("ESE");
        assert!(result.is_ok());
    }

    #[test]
    fn load_portfolio() {
        let mut referential = Referential::new(concat!(env!("CARGO_MANIFEST_DIR"), "/data"));
        let result = referential.load_portfolio(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/example_01.json"
        ));
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.name == "Example 01");
        assert!(result.currency.name == "EUR");
        let instruments = result.get_instrument_name_list();
        assert!(instruments.len() == 2);
        assert!(instruments.contains(&String::from("ESE")));
        assert!(instruments.contains(&String::from("RS2K")));
        let date = result.get_trade_date();
        assert!(date.is_ok());
        let date = date.unwrap();
        assert!(
            date == chrono::NaiveDate::from_ymd_opt(2022, 3, 1).unwrap(),
            "get_trade_date={}",
            date
        );
    }
}
