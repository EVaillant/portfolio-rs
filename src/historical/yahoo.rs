use super::{DataFrame, Requester};
use crate::alias::Date;
use crate::error::Error;
use crate::marketdata::Instrument;

use chrono::Timelike;
use log::{debug, info};
use yahoo_finance_api::{Interval, YahooBuilder};

pub struct YahooRequester;

impl YahooRequester {
    fn request_data(&self, ticker: &str, begin: Date, end: Date) -> Result<Vec<DataFrame>, Error> {
        let request_result = YahooBuilder::new()
            .set_period(begin, end)
            .set_interval(Interval::Day1)
            .set_ticker(ticker)
            .request_chart()?;

        let mut data_frames: Vec<DataFrame> = Vec::new();
        for (instrument_position, result) in request_result.chart.result.iter().enumerate() {
            if result.timestamp.is_none() {
                continue;
            }

            let quotes = result
                .indicators
                .quote
                .get(instrument_position)
                .ok_or_else(|| {
                    Error::new_historical(format!(
                        "unable to get quote at instrument_position:{}",
                        instrument_position
                    ))
                })?;

            for (date_position, date) in result.timestamp.as_ref().unwrap().iter().enumerate() {
                if date.hour() > 8 || date.minute() != 0 || date.second() != 0 {
                    debug!("skip {} because not a real close", date);
                    continue;
                }
                if quotes.open.is_none()
                    || quotes.close.is_none()
                    || quotes.high.is_none()
                    || quotes.low.is_none()
                {
                    continue;
                }
                let open = quotes
                    .open
                    .as_ref()
                    .unwrap()
                    .get(date_position)
                    .ok_or_else(|| {
                        Error::new_historical(format!(
                            "unable to get open at instrument_position:{} date_position:{}",
                            instrument_position, date_position
                        ))
                    })?;
                let close = quotes
                    .close
                    .as_ref()
                    .unwrap()
                    .get(date_position)
                    .ok_or_else(|| {
                        Error::new_historical(format!(
                            "unable to get close at instrument_position:{} date_position:{}",
                            instrument_position, date_position
                        ))
                    })?;
                let high = quotes
                    .high
                    .as_ref()
                    .unwrap()
                    .get(date_position)
                    .ok_or_else(|| {
                        Error::new_historical(format!(
                            "unable to get high at instrument_position:{} date_position:{}",
                            instrument_position, date_position
                        ))
                    })?;
                let low = quotes
                    .low
                    .as_ref()
                    .unwrap()
                    .get(date_position)
                    .ok_or_else(|| {
                        Error::new_historical(format!(
                            "unable to get low at instrument_position:{} date_position:{}",
                            instrument_position, date_position
                        ))
                    })?;
                if open.is_some() && close.is_some() && high.is_some() && low.is_some() {
                    data_frames.push(DataFrame::new(
                        date.date(),
                        open.unwrap(),
                        close.unwrap(),
                        high.unwrap(),
                        low.unwrap(),
                    ));
                } else {
                    info!("value not available at {}", date);
                }
            }
        }
        Ok(data_frames)
    }
}

impl Requester for YahooRequester {
    fn request(
        &self,
        instrument: &Instrument,
        begin: Date,
        end: Date,
    ) -> Result<(Date, Date, Vec<DataFrame>), Error> {
        info!(
            "try to request historic data for {} between {} to {}",
            instrument.name,
            begin.format("%Y-%m-%d"),
            end.format("%Y-%m-%d")
        );
        let end = end
            .checked_add_days(chrono::Days::new(1))
            .ok_or_else(|| Error::new_historical(format!("unable to compute next day {}", end)))?;

        let ticker_yahoo = instrument.ticker_yahoo.as_ref().ok_or_else(|| {
            Error::new_historical(format!("missing yahoo ticker on {}", instrument.name))
        })?;
        debug!("request historic data for {}", instrument.name);
        let result = self.request_data(ticker_yahoo, begin, end)?;
        let result_begin;
        let result_end;
        if result.is_empty() {
            result_begin = Default::default();
            result_end = Default::default();
        } else {
            result_begin = result.first().unwrap().date;
            result_end = result.last().unwrap().date;
        }
        info!("request historic data for {} done", instrument.name);
        Ok((result_begin, result_end, result))
    }
}
