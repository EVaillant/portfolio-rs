use crate::alias::Date;
use log::info;
use std::collections::HashMap;
use std::result;

use crate::error::Error;
use crate::marketdata::Instrument;

mod yahoo;
pub use yahoo::*;

pub trait DataFrame {
    fn new(date: Date, open: f64, close: f64, low: f64, high: f64) -> Self;
    fn date(&self) -> &Date;
    fn open(&self) -> f64;
    fn close(&self) -> f64;
    fn low(&self) -> f64;
    fn high(&self) -> f64;
}

pub trait Provider {
    type DataFrame;

    fn request(
        &self,
        instrument: &Instrument,
        begin: Date,
        end: Date,
    ) -> Result<Vec<Self::DataFrame>, Error>;
}

struct DataCache<P>
where
    P: Provider,
{
    begin: Date,
    end: Date,
    datas: Vec<P::DataFrame>,
}

pub struct HistoricalData<P>
where
    P: Provider,
    <P as Provider>::DataFrame: DataFrame,
{
    provider: P,
    cache: HashMap<String, DataCache<P>>,
}

pub trait Persistance {
    fn save<P>(&self, instrument: &Instrument, datas: &[P]) -> Result<(), Error>
    where
        P: DataFrame;

    fn load<P>(&self, instrument: &Instrument) -> Result<Option<(Date, Date, Vec<P>)>, Error>
    where
        P: DataFrame;
}

impl<P> HistoricalData<P>
where
    P: Provider,
    <P as Provider>::DataFrame: DataFrame,
{
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            cache: Default::default(),
        }
    }

    pub fn request<E: Persistance>(
        &mut self,
        persistence: &E,
        instrument: &Instrument,
        begin: Date,
        end: Date,
    ) -> Result<(), Error> {
        info!(
            "try to request historic data for {} between {} to {}",
            instrument.name,
            begin.format("%Y-%m-%d"),
            end.format("%Y-%m-%d")
        );
        let result = self.request_(persistence, instrument, begin, end);
        info!("request historic data for {} done", instrument.name);
        result
    }

    fn request_<E: Persistance>(
        &mut self,
        persistence: &E,
        instrument: &Instrument,
        begin: Date,
        end: Date,
    ) -> Result<(), Error> {
        let cache_item = self.cache.get_mut(&instrument.name);
        if let Some(data_cache) = cache_item {
            if begin < data_cache.begin {
                let mut result = self.provider.request(instrument, begin, data_cache.begin)?;
                persistence.save(instrument, &result)?;
                data_cache.begin = begin;
                result.append(&mut data_cache.datas);
                data_cache.datas = result;
            }
            if end > data_cache.end {
                let mut result = self.provider.request(instrument, data_cache.end, end)?;
                persistence.save(instrument, &result)?;
                data_cache.end = end;
                data_cache.datas.append(&mut result);
            }
        } else {
            info!(
                "try to request historic data for {} from persistence",
                instrument.name
            );
            if let Some(db_result) = persistence.load(instrument)? {
                let item = DataCache::<P> {
                    begin: db_result.0,
                    end: db_result.1,
                    datas: db_result.2,
                };
                info!(
                    "historic data for {} from persistence found begin:{} end:{} nb_record:{}",
                    instrument.name,
                    item.begin.format("%Y-%m-%d"),
                    item.end.format("%Y-%m-%d"),
                    item.datas.len()
                );
                self.cache.insert(instrument.name.clone(), item);
                return self.request_(persistence, instrument, begin, end);
            } else {
                info!("no historic data for {} from persistence", instrument.name);
                let result = self.provider.request(instrument, begin, end)?;
                persistence.save(instrument, &result)?;
                let item = DataCache::<P> {
                    begin,
                    end,
                    datas: result,
                };
                self.cache.insert(instrument.name.clone(), item);
            }
        }
        Ok(())
    }

    pub fn iter(&self, instrument: &Instrument) -> Option<std::slice::Iter<P::DataFrame>> {
        self.cache
            .get(&instrument.name)
            .map(|item| item.datas.iter())
    }
}
