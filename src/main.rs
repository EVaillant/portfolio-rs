use clap::Parser;
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

use historical::{HistoricalData, YahooRequester};
use persistence::SQLitePersistance;
use portfolio::Portfolio;
use pricer::{PortfolioIndicators, Step};
use referential::Referential;

use crate::error::Error;

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
    let requester = YahooRequester::new()?;
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
    let step_filename = match step {
        Step::Day => "daily",
        Step::Month => "monthly",
        Step::Week => "weekly",
        Step::Year => "yearly",
    };

    let filename =
        String::from(output_dir) + "/indicators_" + step_filename + "_" + &portfolio.name + ".csv";
    indicators.dump_indicators_in_csv(&filename)?;

    for instrument_name in portfolio.get_instrument_name_list().iter() {
        let filename = String::from(output_dir)
            + "/indicators_"
            + step_filename
            + "_"
            + &portfolio.name
            + "_"
            + instrument_name
            + ".csv";

        indicators.dump_instrument_indicators_in_csv(instrument_name, &filename)?;
    }

    Ok(())
}
