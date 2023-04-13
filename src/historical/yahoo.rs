use super::{DataFrame, Requester};
use crate::alias::Date;
use crate::error::{Error, ErrorKind};
use crate::marketdata::Instrument;

use log::{debug, info, warn};
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use std::io::Read;

#[derive(Debug, Deserialize)]
pub struct YahooDataFrame {
    #[serde(rename = "Date")]
    #[serde(deserialize_with = "deserialize_date")]
    date: Date,
    #[serde(rename = "Open")]
    open: f64,
    #[serde(rename = "High")]
    high: f64,
    #[serde(rename = "Low")]
    low: f64,
    #[serde(rename = "Close")]
    close: f64,
}

impl From<YahooDataFrame> for DataFrame {
    fn from(value: YahooDataFrame) -> Self {
        DataFrame {
            date: value.date,
            open: value.open,
            high: value.high,
            low: value.low,
            close: value.close,
        }
    }
}

fn deserialize_date<'de, D>(deserializer: D) -> Result<Date, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    let result =
        chrono::NaiveDate::parse_from_str(&buf, "%Y-%m-%d").map_err(serde::de::Error::custom)?;
    Ok(result)
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
            .map_err(|error| {
                Error::new(
                    ErrorKind::Historical,
                    format!("failed to init reqwest : {error}"),
                )
            })?;

        Ok(Self {
            reqwest_client: client,
        })
    }

    fn request_crumb(&self, ticker: &String) -> Result<String, Error> {
        let mut body = String::new();
        self.reqwest_client
            .get(format!("https://finance.yahoo.com/quote/{ticker}/history"))
            .send()
            .map_err(|error| {
                Error::new(
                    ErrorKind::Historical,
                    format!(
                        "request failed to get history to have crumb ticker:{ticker} error:{error}"
                    ),
                )
            })?
            .read_to_string(&mut body)
            .map_err(|error| {
                Error::new(
                    ErrorKind::Historical,
                    format!(
                        "request failed to get body of history to have crumb ticker:{ticker} error:{error}"),
                )
            })?;
        let re = Regex::new(r#""CrumbStore":\{"crumb":"(.+?)"\}"#).unwrap();
        let mut crumb: String = "".to_string();
        for cap in re.captures_iter(&body) {
            crumb = cap[0].to_string();
        }
        Ok(crumb.chars().skip(23).take(11).collect())
    }

    fn request_data(
        &self,
        ticker: &str,
        crumb: &str,
        begin: Date,
        end: Date,
    ) -> Result<Vec<DataFrame>, Error> {
        let output = self.reqwest_client.get(format!("https://query1.finance.yahoo.com/v7/finance/download/{}?period1={}&period2={}&interval=1d&events=history&crumb={}", ticker, begin.and_hms_opt(0, 0, 0).unwrap().timestamp(), end.and_hms_opt(0, 0, 0).unwrap().timestamp(), crumb))
        .send()
        .map_err(|error| {
            Error::new(ErrorKind::Historical,format!("failed to request historic ticker:{ticker} error:{error}"))
        })?
        .text()
        .map_err(|error| {
            Error::new(ErrorKind::Historical,format!(
                "failed to read body from request historic ticker:{ticker} error:{error}"))
        })?;
        let mut csv_reader = csv::Reader::from_reader(output.as_bytes());
        let mut data_frames: Vec<DataFrame> = Vec::new();
        for result in csv_reader.deserialize::<YahooDataFrame>() {
            match result {
                Ok(record) => data_frames.push(record.into()),
                Err(error) => {
                    warn!("invalid csv format ticker:{ticker} error:{error}");
                }
            };
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
        let ticker_yahoo = instrument.ticker_yahoo.as_ref().ok_or_else(|| {
            Error::new(
                ErrorKind::Historical,
                format!("missing yahoo ticker on {}", instrument.name),
            )
        })?;
        debug!("try to request crumb for {}", instrument.name);
        let crumb = self.request_crumb(ticker_yahoo)?;
        debug!("request crumb {} for {} done", crumb, instrument.name);
        let result = self.request_data(ticker_yahoo, &crumb, begin, end)?;
        let result_begin;
        let result_end;
        if result.is_empty() {
            result_begin = Default::default();
            result_end = Default::default();
        } else {
            result_begin = *result.first().unwrap().date();
            result_end = *result.last().unwrap().date();
        }
        info!("request historic data for {} done", instrument.name);
        Ok((result_begin, result_end, result))
    }
}
