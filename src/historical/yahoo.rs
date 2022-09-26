use super::{DataFrame, Provider};
use crate::alias::Date;
use crate::error::{Error, ErrorKind};
use crate::marketdata::Instrument;

use log::{debug, info};
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use std::io::Read;

#[derive(Debug, Deserialize)]
pub struct YahooDataFrame {
    #[serde(rename = "Date")]
    #[serde(deserialize_with = "deserialize_date")]
    pub date: Date,
    #[serde(rename = "Open")]
    pub open: f64,
    #[serde(rename = "High")]
    pub high: f64,
    #[serde(rename = "Low")]
    pub low: f64,
    #[serde(rename = "Close")]
    pub close: f64,
}

fn deserialize_date<'de, D>(deserializer: D) -> Result<Date, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    let result =
        chrono::NaiveDate::parse_from_str(&buf, "%Y-%m-%d").map_err(serde::de::Error::custom)?;
    Ok(Date::from_utc(result, chrono::Utc))
}

impl DataFrame for YahooDataFrame {
    fn new(date: Date, open: f64, close: f64, low: f64, high: f64) -> Self {
        Self {
            date,
            open,
            high,
            low,
            close,
        }
    }
    fn date(&self) -> &Date {
        &self.date
    }

    fn open(&self) -> f64 {
        self.open
    }

    fn close(&self) -> f64 {
        self.close
    }

    fn low(&self) -> f64 {
        self.low
    }

    fn high(&self) -> f64 {
        self.high
    }
}

pub struct YahooProvider {
    reqwest_client: Client,
}

impl YahooProvider {
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
                    format!("failed to init reqwest : {}", error),
                )
            })?;

        Ok(Self {
            reqwest_client: client,
        })
    }

    fn request_crumb(&self, ticker: &String) -> Result<String, Error> {
        let mut body = String::new();
        self.reqwest_client
            .get(format!(
                "https://finance.yahoo.com/quote/{}/history",
                ticker
            ))
            .send()
            .map_err(|error| {
                Error::new(
                    ErrorKind::Historical,
                    format!(
                        "request failed to get history to have crumb ticker:{} error:{}",
                        ticker, error
                    ),
                )
            })?
            .read_to_string(&mut body)
            .map_err(|error| {
                Error::new(
                    ErrorKind::Historical,
                    format!(
                        "request failed to get body of history to have crumb ticker:{} error:{}",
                        ticker, error
                    ),
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
    ) -> Result<Vec<YahooDataFrame>, Error> {
        let output = self.reqwest_client.get(format!("https://query1.finance.yahoo.com/v7/finance/download/{}?period1={}&period2={}&interval=1d&events=history&crumb={}", ticker, begin.and_hms(0, 0, 0).timestamp(), end.and_hms(0, 0, 0).timestamp(), crumb))
        .send()
        .map_err(|error| {
            Error::new(ErrorKind::Historical,format!("failed to request historic ticker:{} error:{}",
                ticker, error
            ))
        })?
        .text()
        .map_err(|error| {
            Error::new(ErrorKind::Historical,format!(
                "failed to read body from request historic ticker:{} error:{}",
                ticker, error
            ))
        })?;
        let mut csv_reader = csv::Reader::from_reader(output.as_bytes());
        let mut data_frames = Vec::new();
        for result in csv_reader.deserialize() {
            let record: YahooDataFrame = result.map_err(|error| {
                Error::new(
                    ErrorKind::Historical,
                    format!(
                        "invalid csv format ticker:{} error:{} csv:{}",
                        ticker, error, output
                    ),
                )
            })?;
            data_frames.push(record);
        }
        Ok(data_frames)
    }
}

impl Provider for YahooProvider {
    type DataFrame = YahooDataFrame;

    fn request(
        &self,
        instrument: &Instrument,
        begin: Date,
        end: Date,
    ) -> Result<Vec<Self::DataFrame>, Error> {
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
        let result = self.request_data(ticker_yahoo, &crumb, begin, end);
        info!("request historic data for {} done", instrument.name);
        result
    }
}
