use super::{DataFrame, Requester};
use crate::alias::Date;
use crate::error::Error;
use crate::marketdata::Instrument;

use log::{debug, info};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct YahooResult {
    chart: YahooChart,
}

#[derive(Debug, Deserialize)]
struct YahooChart {
    result: Vec<YahooChartResult>,
}

#[derive(Debug, Deserialize)]
struct YahooChartResult {
    #[serde(deserialize_with = "deserialize_vec_timestamp")]
    timestamp: Vec<Date>,
    indicators: YahooChartIndicators,
}

#[derive(Debug, Deserialize)]
struct YahooChartIndicators {
    quote: Vec<YahooChartQuote>,
}

#[derive(Debug, Deserialize)]
struct YahooChartQuote {
    low: Vec<Option<f64>>,
    open: Vec<Option<f64>>,
    close: Vec<Option<f64>>,
    high: Vec<Option<f64>>,
    volume: Vec<Option<f64>>,
}

fn deserialize_vec_timestamp<'de, D>(deserializer: D) -> Result<Vec<Date>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values: Vec<i64> = Vec::deserialize(deserializer)?;
    let mut dates = Vec::with_capacity(values.len());
    for value in values {
        let date = chrono::DateTime::from_timestamp(value, 0)
            .ok_or_else(|| {
                serde::de::Error::custom(format!("unable to create date from timestamp {}", value))
            })?
            .naive_local()
            .date();
        dates.push(date);
    }
    Ok(dates)
}

pub struct YahooRequester {
    reqwest_client: Client,
}

impl YahooRequester {
    pub fn new() -> Result<Self, Error> {
        let mut headers = HeaderMap::new();
        headers.insert("Connection", HeaderValue::from_static("keep-alive"));
        headers.insert("Expires", HeaderValue::from_static("-1"));
        headers.insert("Upgrade-Insecure-Requests", HeaderValue::from_static("1"));
        headers.insert("User-Agent", HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/54.0.2840.99 Safari/537.36"));

        let client = Client::builder()
            .cookie_store(true)
            .default_headers(headers)
            .build()
            .map_err(|error| Error::new_historical(format!("failed to init reqwest : {error}")))?;

        Ok(Self {
            reqwest_client: client,
        })
    }

    fn request_data(&self, ticker: &str, begin: Date, end: Date) -> Result<Vec<DataFrame>, Error> {
        let url = format!("https://query1.finance.yahoo.com/v8/finance/chart/{}?period1={}&period2={}&interval=1d", ticker, begin.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp(), end.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp());
        debug!("request data from url {}", url);
        let output = self
            .reqwest_client
            .get(url)
            .send()
            .map_err(|error| {
                Error::new_historical(format!(
                    "failed to request historic ticker:{ticker} error:{error}"
                ))
            })?
            .text()
            .map_err(|error| {
                Error::new_historical(format!(
                    "failed to read body from request historic ticker:{ticker} error:{error}"
                ))
            })?;
        debug!("request result: {}", output);
        let request_result: YahooResult = serde_json::from_reader(output.as_bytes())?;
        let mut data_frames: Vec<DataFrame> = Vec::new();
        for (instrument_position, result) in request_result.chart.result.iter().enumerate() {
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
            for (date_position, date) in result.timestamp.iter().enumerate() {
                let open = quotes.open.get(date_position).ok_or_else(|| {
                    Error::new_historical(format!(
                        "unable to get open at instrument_position:{} date_position:{}",
                        instrument_position, date_position
                    ))
                })?;
                let close = quotes.close.get(date_position).ok_or_else(|| {
                    Error::new_historical(format!(
                        "unable to get close at instrument_position:{} date_position:{}",
                        instrument_position, date_position
                    ))
                })?;
                let high = quotes.high.get(date_position).ok_or_else(|| {
                    Error::new_historical(format!(
                        "unable to get high at instrument_position:{} date_position:{}",
                        instrument_position, date_position
                    ))
                })?;
                let low = quotes.low.get(date_position).ok_or_else(|| {
                    Error::new_historical(format!(
                        "unable to get low at instrument_position:{} date_position:{}",
                        instrument_position, date_position
                    ))
                })?;
                if open.is_some() && close.is_some() && high.is_some() && low.is_some() {
                    data_frames.push(DataFrame::new(
                        *date,
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
