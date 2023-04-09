use crate::alias::{Date, DateTime};
use crate::error::{Error, ErrorKind};
use crate::marketdata::{Currency, Dividend, Instrument, Market, ParentCurrency};
use crate::portfolio::{CashVariation, CashVariationSource, Portfolio, Position, Trade, Way};

use serde_json::Value;
use std::rc::Rc;

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::new(ErrorKind::Referential, format!("not json format : {error}"))
    }
}

pub trait Resolver {
    fn resolv_currency(&mut self, name: &str) -> Result<Rc<Currency>, Error>;
    fn resolv_market(&mut self, name: &str) -> Result<Rc<Market>, Error>;
    fn resolv_instrument(&mut self, name: &str) -> Result<Rc<Instrument>, Error>;
}

pub trait Deserialize: Sized {
    fn deserialize<D>(deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer;
}

pub trait Deserializer {
    fn read<T>(&mut self, name: &str) -> Result<T, Error>
    where
        T: Deserialize;

    fn read_option<T>(&mut self, name: &str) -> Result<Option<T>, Error>
    where
        T: Deserialize;

    fn read_array<T>(&mut self) -> Result<Vec<T>, Error>
    where
        T: Deserialize;

    fn read_string(&self) -> Result<String, Error>;
    fn read_f64(&self) -> Result<f64, Error>;
    fn read_u64(&self) -> Result<u64, Error>;

    fn resolv_currency(&mut self, name: &str) -> Result<Rc<Currency>, Error>;
    fn resolv_market(&mut self, name: &str) -> Result<Rc<Market>, Error>;
    fn resolv_instrument(&mut self, name: &str) -> Result<Rc<Instrument>, Error>;
}

pub struct DeserializerValue<'a, R: Resolver> {
    value: &'a Value,
    resolver: &'a mut R,
}

impl<'a, R: Resolver> Deserializer for DeserializerValue<'a, R> {
    fn read<T>(&mut self, name: &str) -> Result<T, Error>
    where
        T: Deserialize,
    {
        let value = self
            .value
            .as_object()
            .ok_or_else(|| Error::new(ErrorKind::Referential, "field must be an object"))?
            .get(name)
            .ok_or_else(|| {
                Error::new(ErrorKind::Referential, format!("field {name} is mandatory"))
            })?;
        let sub_deserializer = DeserializerValue {
            value,
            resolver: self.resolver,
        };
        T::deserialize(sub_deserializer)
    }

    fn read_option<T>(&mut self, name: &str) -> Result<Option<T>, Error>
    where
        T: Deserialize,
    {
        self.value
            .as_object()
            .ok_or_else(|| Error::new(ErrorKind::Referential, "field must be an object"))?
            .get(name)
            .map(|value| {
                let sub_deserializer = DeserializerValue {
                    value,
                    resolver: self.resolver,
                };
                T::deserialize(sub_deserializer)
            })
            .transpose()
    }

    fn read_string(&self) -> Result<String, Error> {
        self.value
            .as_str()
            .map(|item| item.to_string())
            .ok_or_else(|| Error::new(ErrorKind::Referential, "field must be a string"))
    }

    fn read_f64(&self) -> Result<f64, Error> {
        self.value
            .as_f64()
            .ok_or_else(|| Error::new(ErrorKind::Referential, "field must be a f64"))
    }

    fn read_u64(&self) -> Result<u64, Error> {
        self.value
            .as_u64()
            .ok_or_else(|| Error::new(ErrorKind::Referential, "field must be a u64"))
    }

    fn read_array<T>(&mut self) -> Result<Vec<T>, Error>
    where
        T: Deserialize,
    {
        self.value
            .as_array()
            .ok_or_else(|| Error::new(ErrorKind::Referential, "field must be an array"))?
            .iter()
            .map(|value| {
                let deserializer = DeserializerValue {
                    value,
                    resolver: self.resolver,
                };
                T::deserialize(deserializer)
            })
            .collect()
    }

    fn resolv_currency(&mut self, name: &str) -> Result<Rc<Currency>, Error> {
        let currency_name: String = self.read(name)?;
        self.resolver.resolv_currency(currency_name.as_str())
    }

    fn resolv_market(&mut self, name: &str) -> Result<Rc<Market>, Error> {
        let market_name: String = self.read(name)?;
        self.resolver.resolv_market(market_name.as_str())
    }

    fn resolv_instrument(&mut self, name: &str) -> Result<Rc<Instrument>, Error> {
        let instrument_name: String = self.read(name)?;
        self.resolver.resolv_instrument(instrument_name.as_str())
    }
}

impl Deserialize for String {
    fn deserialize<D>(deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        deserializer.read_string()
    }
}
impl Deserialize for f32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        deserializer.read_f64().map(|item| item as f32)
    }
}

impl Deserialize for f64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        deserializer.read_f64()
    }
}

impl Deserialize for u32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        deserializer.read_u64().map(|item| item as u32)
    }
}

impl Deserialize for u64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        deserializer.read_u64()
    }
}

impl Deserialize for Way {
    fn deserialize<D>(deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        let value: String = deserializer.read_string()?;
        match value.as_str() {
            "buy" => Ok(Self::Buy),
            "sell" => Ok(Self::Sell),
            _ => Err(Error::new(
                ErrorKind::Referential,
                format!("unable to convert {value} into Way"),
            )),
        }
    }
}

impl Deserialize for CashVariationSource {
    fn deserialize<D>(deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        let value: String = deserializer.read_string()?;
        match value.as_str() {
            "trade" => Ok(Self::Trade),
            "payement" => Ok(Self::Payement),
            "dividend" => Ok(Self::Dividend),
            _ => Err(Error::new(
                ErrorKind::Referential,
                format!("unable to convert {value} into CashVariationSource"),
            )),
        }
    }
}

impl Deserialize for DateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        let value: String = deserializer.read_string()?;
        let result = chrono::DateTime::parse_from_rfc3339(value.as_str());
        match result {
            Ok(value) => Ok(value.naive_local()),
            Err(err) => Err(Error::new(
                ErrorKind::Referential,
                format!("unable to convert {value} into Date because {err}"),
            )),
        }
    }
}

impl Deserialize for Date {
    fn deserialize<D>(deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        let value: String = deserializer.read_string()?;
        let result = chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d");
        match result {
            Ok(value) => Ok(value),
            Err(err) => Err(Error::new(
                ErrorKind::Referential,
                format!("unable to convert {value} into Date because {err}"),
            )),
        }
    }
}

impl Deserialize for Trade {
    fn deserialize<D>(mut deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        let date = deserializer.read("date")?;
        let way = deserializer.read("way")?;
        let quantity = deserializer.read("quantity")?;
        let price = deserializer.read("price")?;
        let tax = deserializer.read("tax")?;
        Ok(Trade {
            date,
            way,
            quantity,
            price,
            tax,
        })
    }
}

impl Deserialize for Position {
    fn deserialize<D>(mut deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        let instrument = deserializer.resolv_instrument("instrument")?;
        let mut trades: Vec<Trade> = deserializer.read("trades")?;
        trades.sort_by(|left, right| left.date.cmp(&right.date));
        Ok(Position { instrument, trades })
    }
}

impl Deserialize for CashVariation {
    fn deserialize<D>(mut deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        let position = deserializer.read("position")?;
        let date = deserializer.read("date")?;
        let source = deserializer.read("source")?;
        Ok(CashVariation {
            position,
            date,
            source,
        })
    }
}

impl Deserialize for Portfolio {
    fn deserialize<D>(mut deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        let name = deserializer.read("name")?;
        let currency = deserializer.resolv_currency("currency")?;
        let positions = deserializer.read("positions")?;
        let mut cash: Vec<CashVariation> = deserializer.read("cash")?;
        cash.sort_by(|left, right| left.date.cmp(&right.date));
        Ok(Portfolio {
            name,
            currency,
            positions,
            cash,
        })
    }
}

impl Deserialize for Market {
    fn deserialize<D>(mut deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        let name = deserializer.read("name")?;
        let description = deserializer.read("description")?;
        Ok(Market { name, description })
    }
}

impl Deserialize for Instrument {
    fn deserialize<D>(mut deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        let name = deserializer.read("name")?;
        let description = deserializer.read("description")?;
        let market = deserializer.resolv_market("market")?;
        let currency = deserializer.resolv_currency("currency")?;
        let ticker_yahoo = deserializer.read_option("ticker_yahoo")?;
        let dividends = deserializer.read_option("dividends")?;
        Ok(Instrument {
            name,
            description,
            market,
            currency,
            ticker_yahoo,
            dividends,
        })
    }
}

impl Deserialize for ParentCurrency {
    fn deserialize<D>(mut deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        let factor = deserializer.read("factor")?;
        let currency = deserializer.resolv_currency("currency")?;
        Ok(ParentCurrency { factor, currency })
    }
}

impl Deserialize for Dividend {
    fn deserialize<D>(mut deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        let record_date = deserializer.read("record_date")?;
        let payment_date = deserializer.read("payment_date")?;
        let value = deserializer.read("value")?;
        Ok(Dividend {
            record_date,
            payment_date,
            value,
        })
    }
}

impl Deserialize for Currency {
    fn deserialize<D>(mut deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        let name = deserializer.read("name")?;
        let parent_currency = deserializer.read_option("parent_currency")?;
        Ok(Currency {
            name,
            parent_currency,
        })
    }
}

impl<T> Deserialize for Vec<T>
where
    T: Deserialize,
{
    fn deserialize<D>(mut deserializer: D) -> Result<Self, Error>
    where
        D: Deserializer,
    {
        deserializer.read_array()
    }
}

pub fn from_reader<R, T, O>(reader: R, resolver: &mut O) -> Result<T, Error>
where
    R: std::io::Read,
    T: Deserialize,
    O: Resolver,
{
    let value: Value = serde_json::from_reader(reader)?;
    let deserializer = DeserializerValue {
        value: &value,
        resolver,
    };
    T::deserialize(deserializer)
}
