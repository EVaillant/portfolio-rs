use clap::{Parser, ValueEnum};
use env_logger::Builder;
use log::info;
use log::LevelFilter;

mod alias;
mod error;
mod historical;
mod marketdata;
mod persistence;
mod portfolio;
mod pricer;
mod referential;

use historical::{HistoricalData, NullRequester, Requester, YahooRequester};
use persistence::SQLitePersistance;
use portfolio::Portfolio;
use pricer::{PortfolioIndicators, Step};
use referential::Referential;

use crate::error::Error;

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

    /// output dir
    #[clap(short, long, value_parser)]
    output_dir: String,

    /// spot source
    #[clap(default_value_t = SpotSource::Yahoo, short, long, value_parser)]
    spot_source: SpotSource,
}

fn make_requester(source: SpotSource) -> Result<Box<dyn Requester>, Error> {
    let value: Box<dyn Requester> = match source {
        SpotSource::Null => Box::new(NullRequester),
        SpotSource::Yahoo => Box::new(YahooRequester::new()?),
    };
    Ok(value)
}

fn main() -> Result<(), Error> {
    //
    // cli arg
    let args = Args::parse();

    //
    // logger
    let mut builder = Builder::new();
    builder.filter_level(LevelFilter::Info);
    builder.parse_default_env();
    builder.init();

    //
    // Load portfolio
    let mut referential = Referential::new(args.marketdata_dir);
    let portfolio = referential.load_portfolio(&args.portfolio)?;
    info!("loading portfolio {} done", portfolio.name);

    //
    // persistence
    let persistence = SQLitePersistance::new(&args.cache_file)?;

    //
    // historical data
    let requester = make_requester(args.spot_source)?;
    let mut provider = HistoricalData::new(requester, &persistence);

    //
    // compute main portfolio
    let compute_begin = portfolio.get_trade_date()?;
    let compute_end = chrono::Utc::now().date_naive();
    for step in vec![Step::Day, Step::Week, Step::Month, Step::Year].iter() {
        let portfolio_indicators = PortfolioIndicators::from_portfolio(
            &portfolio,
            compute_begin,
            compute_end,
            *step,
            &mut provider,
        )?;

        //
        // dump output
        dump_portfolio_indicators(&args.output_dir, &portfolio, &portfolio_indicators, *step)?;
    }
    Ok(())
}

fn dump_portfolio_indicators(
    output_dir: &str,
    portfolio: &Portfolio,
    indicators: &PortfolioIndicators,
    step: Step,
) -> Result<(), Error> {
    let filename = String::from(output_dir)
        + "/indicators_"
        + step.to_string()
        + "_"
        + &portfolio.name
        + ".csv";
    indicators.dump_indicators_in_csv(&filename)?;

    for instrument_name in portfolio.get_instrument_name_list().iter() {
        let filename = String::from(output_dir)
            + "/indicators_"
            + step.to_string()
            + "_"
            + &portfolio.name
            + "_"
            + instrument_name
            + ".csv";

        indicators.dump_instrument_indicators_in_csv(instrument_name, &filename)?;
    }

    Ok(())
}
