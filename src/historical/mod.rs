use crate::alias::Date;
use log::info;
use std::collections::HashMap;

use crate::error::Error;
use crate::marketdata::Instrument;

mod yahoo;
pub use yahoo::*;

#[derive(Copy, Clone)]
pub struct DataFrame {
    pub date: Date,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
}

impl DataFrame {
    #[inline]
    pub fn new(date: Date, open: f64, close: f64, high: f64, low: f64) -> Self {
        Self {
            date,
            open,
            close,
            high,
            low,
        }
    }
}

pub trait Provider {
    fn fetch(&mut self, instrument: &Instrument, begin: Date, end: Date) -> Result<(), Error>;
    fn latest(&self, instrument: &Instrument, date: Date) -> Option<&DataFrame>;
}

pub trait Requester {
    fn request(
        &self,
        instrument: &Instrument,
        begin: Date,
        end: Date,
    ) -> Result<(Date, Date, Vec<DataFrame>), Error>;
}

pub struct NullRequester;
impl Requester for NullRequester {
    fn request(
        &self,
        _instrument: &Instrument,
        _begin: Date,
        _end: Date,
    ) -> Result<(Date, Date, Vec<DataFrame>), Error> {
        Ok((Default::default(), Default::default(), Default::default()))
    }
}

pub trait Persistance {
    fn save(&self, instrument: &Instrument, datas: &[DataFrame]) -> Result<(), Error>;
    fn load(&self, instrument: &Instrument) -> Result<Option<(Date, Date, Vec<DataFrame>)>, Error>;
}

struct CacheInstrument {
    begin: Date,
    end: Date,
    data: Vec<DataFrame>,
}

impl CacheInstrument {
    fn new(begin: Date, end: Date, data: Vec<DataFrame>) -> Self {
        Self { begin, end, data }
    }

    fn latest(&self, date: Date) -> Option<&DataFrame> {
        self.data.iter().rev().find(|item| item.date <= date)
    }

    fn insert(&mut self, begin: Date, end: Date, mut data: Vec<DataFrame>) {
        if begin < self.begin {
            if end > self.end {
                self.begin = begin;
                self.end = end;
                self.data = data;
            } else {
                self.begin = begin;
                data.append(&mut self.data);
                self.data = data;
            }
        } else if end > self.end {
            self.data.append(&mut data);
            self.end = end;
        }
    }

    fn not_in_cache(&self, begin: Date, end: Date) -> Option<(Date, Date)> {
        if begin < self.begin {
            if end > self.end {
                Some((begin, end))
            } else {
                Some((begin, self.end))
            }
        } else if end > self.end {
            Some((
                self.end
                    .checked_add_days(chrono::naive::Days::new(1))
                    .unwrap(),
                end,
            ))
        } else {
            None
        }
    }
}

pub struct HistoricalData<'a, P>
where
    P: Persistance,
{
    requester: Box<dyn Requester>,
    persistence: &'a P,
    cache: HashMap<String, CacheInstrument>,
}

impl<'a, P> HistoricalData<'a, P>
where
    P: Persistance,
{
    pub fn new(requester: Box<dyn Requester>, persistence: &'a P) -> Self {
        Self {
            requester,
            persistence,
            cache: Default::default(),
        }
    }

    fn make_cache_key(instrument: &Instrument) -> String {
        instrument.name.clone()
    }
}

impl<P> Provider for HistoricalData<'_, P>
where
    P: Persistance,
{
    fn fetch(&mut self, instrument: &Instrument, begin: Date, end: Date) -> Result<(), Error> {
        info!(
            "try to fetch historic data for {} between {} to {}",
            instrument.name,
            begin.format("%Y-%m-%d"),
            end.format("%Y-%m-%d")
        );

        let key = Self::make_cache_key(instrument);
        let mut cache_item = self.cache.get_mut(&key);
        if cache_item.is_none() {
            if let Some((db_begin, db_end, db_result)) = self.persistence.load(instrument)? {
                info!(
                    "historic data for {} from persistence found begin:{} end:{} nb_record:{}",
                    instrument.name,
                    db_begin.format("%Y-%m-%d"),
                    db_end.format("%Y-%m-%d"),
                    db_result.len()
                );

                let item = CacheInstrument::new(db_begin, db_end, db_result);
                self.cache.insert(key.clone(), item);
                cache_item = self.cache.get_mut(&key);
            }
        }

        let mut request_begin = begin;
        let mut request_end = end;
        if let Some(data_cache) = &cache_item {
            match data_cache.not_in_cache(begin, end) {
                Some((cache_begin, cache_end)) => {
                    request_begin = cache_begin;
                    request_end = cache_end;
                }
                None => {
                    info!("historic data for {} up to date.", instrument.name);
                    return Ok(());
                }
            };
        }

        info!(
            "historic data for {} request from provider begin:{} end:{}",
            instrument.name,
            request_begin.format("%Y-%m-%d"),
            request_end.format("%Y-%m-%d")
        );

        let (result_begin, result_end, result_data) =
            self.requester
                .request(instrument, request_begin, request_end)?;

        if !result_data.is_empty() {
            info!(
                "historic data for {} from provider found begin:{} end:{} nb_record:{}",
                instrument.name,
                result_begin.format("%Y-%m-%d"),
                result_end.format("%Y-%m-%d"),
                result_data.len()
            );

            self.persistence.save(instrument, &result_data)?;

            if let Some(data_cache) = cache_item {
                data_cache.insert(request_begin, request_end, result_data);
            } else {
                let item = CacheInstrument::new(request_begin, request_end, result_data);
                self.cache.insert(key.clone(), item);
            }
        } else {
            info!(
                "historic data for {} from provider return empty",
                instrument.name,
            );
        }

        Ok(())
    }

    fn latest(&self, instrument: &Instrument, date: Date) -> Option<&DataFrame> {
        match self.cache.get(&instrument.name) {
            Some(item) => item.latest(date),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_instrument_01() {
        let begin = make_date_(2022, 5, 1);
        let end = make_date_(2022, 5, 5);
        let data = vec![
            make_dataframe_(2022, 5, 1),
            make_dataframe_(2022, 5, 2),
            make_dataframe_(2022, 5, 3),
            make_dataframe_(2022, 5, 4),
            make_dataframe_(2022, 5, 5),
        ];
        let cache_instrument = CacheInstrument::new(begin, end, data);
        {
            let test_begin = make_date_(2022, 2, 1);
            let test_end = make_date_(2022, 4, 1);
            check_not_in_cache_ok_(&cache_instrument, test_begin, test_end, test_begin, end);
        }
        {
            let test_begin = make_date_(2022, 2, 1);
            let test_end = make_date_(2022, 5, 1);
            check_not_in_cache_ok_(&cache_instrument, test_begin, test_end, test_begin, end);
        }
        {
            let test_begin = make_date_(2022, 2, 1);
            let test_end = make_date_(2022, 5, 5);
            check_not_in_cache_ok_(&cache_instrument, test_begin, test_end, test_begin, end);
        }
        {
            let test_begin = make_date_(2022, 5, 2);
            let test_end = make_date_(2022, 6, 1);
            check_not_in_cache_ok_(
                &cache_instrument,
                test_begin,
                test_end,
                make_date_(2022, 5, 6),
                test_end,
            );
        }
        {
            let test_begin = make_date_(2022, 5, 5);
            let test_end = make_date_(2022, 6, 1);
            check_not_in_cache_ok_(
                &cache_instrument,
                test_begin,
                test_end,
                make_date_(2022, 5, 6),
                test_end,
            );
        }
        {
            let test_begin = make_date_(2022, 5, 10);
            let test_end = make_date_(2022, 6, 1);
            check_not_in_cache_ok_(
                &cache_instrument,
                test_begin,
                test_end,
                make_date_(2022, 5, 6),
                test_end,
            );
        }
        {
            let test_begin = make_date_(2022, 5, 2);
            let test_end = make_date_(2022, 5, 4);
            let result = cache_instrument.not_in_cache(test_begin, test_end);
            assert!(result.is_none());
        }
    }

    #[test]
    fn cache_instrument_02() {
        let begin = make_date_(2022, 5, 1);
        let end = make_date_(2022, 5, 2);
        {
            let mut cache_instrument = CacheInstrument::new(
                begin,
                end,
                vec![make_dataframe_(2022, 5, 1), make_dataframe_(2022, 5, 2)],
            );
            cache_instrument.insert(
                make_date_(2022, 4, 29),
                make_date_(2022, 4, 30),
                vec![make_dataframe_(2022, 4, 29), make_dataframe_(2022, 4, 30)],
            );
            assert_eq!(cache_instrument.data.len(), 4);
            assert_eq!(cache_instrument.data[0].date, make_date_(2022, 4, 29));
            assert_eq!(cache_instrument.data[1].date, make_date_(2022, 4, 30));
            assert_eq!(cache_instrument.data[2].date, make_date_(2022, 5, 1));
            assert_eq!(cache_instrument.data[3].date, make_date_(2022, 5, 2));
        }
        {
            let mut cache_instrument = CacheInstrument::new(
                begin,
                end,
                vec![make_dataframe_(2022, 5, 1), make_dataframe_(2022, 5, 2)],
            );
            cache_instrument.insert(
                make_date_(2022, 5, 3),
                make_date_(2022, 5, 4),
                vec![make_dataframe_(2022, 5, 3), make_dataframe_(2022, 5, 4)],
            );
            assert_eq!(cache_instrument.data.len(), 4);
            assert_eq!(cache_instrument.data[0].date, make_date_(2022, 5, 1));
            assert_eq!(cache_instrument.data[1].date, make_date_(2022, 5, 2));
            assert_eq!(cache_instrument.data[2].date, make_date_(2022, 5, 3));
            assert_eq!(cache_instrument.data[3].date, make_date_(2022, 5, 4));
        }
        {
            let mut cache_instrument = CacheInstrument::new(
                begin,
                end,
                vec![make_dataframe_(2022, 5, 1), make_dataframe_(2022, 5, 2)],
            );
            cache_instrument.insert(
                make_date_(2022, 4, 30),
                make_date_(2022, 5, 3),
                vec![
                    make_dataframe_(2022, 4, 30),
                    make_dataframe_(2022, 5, 1),
                    make_dataframe_(2022, 5, 2),
                    make_dataframe_(2022, 5, 3),
                ],
            );
            assert_eq!(cache_instrument.data.len(), 4);
            assert_eq!(cache_instrument.data[0].date, make_date_(2022, 4, 30));
            assert_eq!(cache_instrument.data[1].date, make_date_(2022, 5, 1));
            assert_eq!(cache_instrument.data[2].date, make_date_(2022, 5, 2));
            assert_eq!(cache_instrument.data[3].date, make_date_(2022, 5, 3));
        }
    }

    #[test]
    fn cache_instrument_03() {
        let cache_instrument = CacheInstrument::new(
            make_date_(2022, 1, 5),
            make_date_(2022, 1, 10),
            vec![
                make_dataframe_(2022, 1, 5),
                make_dataframe_(2022, 1, 6),
                make_dataframe_(2022, 1, 7),
                make_dataframe_(2022, 1, 10),
            ],
        );
        {
            let result = cache_instrument.latest(make_date_(2022, 1, 2));
            assert!(result.is_none());
        }
        {
            let result = cache_instrument.latest(make_date_(2022, 1, 6));
            assert!(result.is_some());
            let dataframe = result.unwrap();
            assert_eq!(dataframe.date, make_date_(2022, 1, 6));
        }
        {
            let result = cache_instrument.latest(make_date_(2022, 1, 8));
            assert!(result.is_some());
            let dataframe = result.unwrap();
            assert_eq!(dataframe.date, make_date_(2022, 1, 7));
        }
        {
            let result = cache_instrument.latest(make_date_(2022, 1, 20));
            assert!(result.is_some());
            let dataframe = result.unwrap();
            assert_eq!(dataframe.date, make_date_(2022, 1, 10));
        }
    }

    fn check_not_in_cache_ok_(
        cache_instrument: &CacheInstrument,
        ibegin: Date,
        iend: Date,
        rbegin: Date,
        rend: Date,
    ) {
        let result = cache_instrument.not_in_cache(ibegin, iend);
        assert!(result.is_some());
        let (result_begin, result_end) = result.unwrap();
        assert_eq!(result_begin, rbegin);
        assert_eq!(result_end, rend);
    }

    fn make_date_(year: i32, month: u32, day: u32) -> Date {
        Date::from_ymd_opt(year, month, day).unwrap()
    }

    fn make_dataframe_(year: i32, month: u32, day: u32) -> DataFrame {
        DataFrame::new(make_date_(year, month, day), 10.0, 10.0, 10.0, 10.0)
    }
}
