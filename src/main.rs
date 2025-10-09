use chrono::Local;
use clap::{Parser, ValueEnum};
use env_logger::Builder;
use log::LevelFilter;
use log::info;
use portfolio::Portfolio;
use std::io::Write;

mod alias;
mod error;
mod historical;
mod marketdata;
mod output;
mod persistence;
mod portfolio;
mod pricer;
mod referential;

use alias::Date;
use historical::{HistoricalData, NullRequester, Requester, YahooRequester};
use output::{CsvOutput, OdsOutput, Output, PortfolioPerformanceOutput};
use persistence::SQLitePersistance;
use pricer::PortfolioIndicators;
use referential::Referential;

use error::Error;

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum SpotSource {
    Null,
    Yahoo,
}

impl std::fmt::Display for SpotSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum OutputType {
    Csv,
    Ods,
    PortfolioPerformance,
}

impl std::fmt::Display for OutputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Marketdata directory
    #[clap(short, long, value_parser)]
    marketdata_dir: String,

    /// Portfolio file
    #[clap(short, long, value_parser)]
    portfolio: String,

    /// db cache file
    #[clap(short, long, value_parser)]
    cache_file: String,

    /// output type
    #[clap(default_value_t = OutputType::Csv, short =  't', long, value_parser)]
    output_type: OutputType,

    /// output
    #[clap(short, long, value_parser)]
    output: String,

    /// spot source
    #[clap(default_value_t = SpotSource::Yahoo, short, long, value_parser)]
    spot_source: SpotSource,

    /// pricing date format YYYY-MM-DD
    #[clap(default_value_t = String::from("now"), short = 'd', long, value_parser)]
    pricing_date: String,

    /// filter output indicator(s)
    #[clap(long, value_parser = parse_indicators_filter)]
    indicators_filter: Option<Date>,

    /// ods details sheet
    #[clap(default_value_t = false, long, value_parser)]
    ods_details_sheet: bool,

    /// ods force rewrite
    #[clap(default_value_t = false, long, value_parser)]
    ods_force_rewrite: bool,
}

fn parse_indicators_filter(arg: &str) -> Result<Date, clap::Error> {
    let days = chrono::naive::Days::new(
        arg.parse()
            .expect("unable to parse to int indicators filter"),
    );
    let previous_date = chrono::Utc::now()
        .date_naive()
        .checked_sub_days(days)
        .expect("unable to compute indicators filter");
    Ok(previous_date)
}

fn make_requester(source: SpotSource) -> Result<Box<dyn Requester>, Error> {
    let value: Box<dyn Requester> = match source {
        SpotSource::Null => Box::new(NullRequester),
        SpotSource::Yahoo => Box::new(YahooRequester),
    };
    Ok(value)
}

fn make_portfolio_indicators(
    args: &Args,
    portfolio: &Portfolio,
) -> Result<PortfolioIndicators, Error> {
    //
    // get pricing date
    let pricing_end_date = if args.pricing_date == "now" {
        chrono::Utc::now().date_naive()
    } else {
        chrono::NaiveDate::parse_from_str(&args.pricing_date, "%Y-%m-%d")
            .expect("invalid pricing date format")
    };

    //
    // persistence
    let persistence = SQLitePersistance::new(&args.cache_file)?;

    //
    // historical data
    let requester = make_requester(args.spot_source)?;
    let mut provider = HistoricalData::new(requester, &persistence);

    //
    // compute main portfolio
    let pricing_begin_date = portfolio.get_trade_date()?;
    let portfolio_indicators = PortfolioIndicators::from_portfolio(
        portfolio,
        pricing_begin_date,
        pricing_end_date,
        &mut provider,
    )?;
    info!("compute portfolio done");
    Ok(portfolio_indicators)
}

fn main() -> Result<(), Error> {
    //
    // cli arg
    let args = Args::parse();

    //
    // logger
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();

    //
    // Load portfolio
    let mut referential = Referential::new(&args.marketdata_dir);
    let portfolio = referential.load_portfolio(&args.portfolio)?;
    info!("loading portfolio {} done", portfolio.name);

    //
    // write output
    match args.output_type {
        OutputType::Csv => {
            let portfolio_indicators = make_portfolio_indicators(&args, &portfolio)?;
            let mut output = CsvOutput::new(
                &args.output,
                &portfolio,
                &portfolio_indicators,
                &args.indicators_filter,
            );
            output.write()?;
        }
        OutputType::Ods => {
            let portfolio_indicators = make_portfolio_indicators(&args, &portfolio)?;
            let mut output = OdsOutput::new(
                &args.output,
                &portfolio,
                &portfolio_indicators,
                &args.indicators_filter,
                args.ods_details_sheet,
                args.ods_force_rewrite,
            )?;
            output.write()?;
        }
        OutputType::PortfolioPerformance => {
            let mut output = PortfolioPerformanceOutput::new(&args.output, &portfolio);
            output.write()?;
        }
    };
    info!("write output done");

    Ok(())
}
