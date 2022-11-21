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

use historical::{HistoricalData, YahooProvider};
use persistence::SQLitePersistance;
use pricer::PortfolioIndicator;
use referential::Referential;

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
}

fn main() {
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
    let portfolio = referential
        .load_portfolio(&args.portfolio)
        .expect("unable to load portfolio");
    info!("loading portfolio {} done", portfolio.name);

    //
    // persistence
    let persistence =
        SQLitePersistance::new(&args.cache_file).expect("failed to create/load persistence file");

    //
    // historical data
    let provider = YahooProvider::new().expect("failed to create yahoo provider");
    let mut histo = HistoricalData::new(provider);

    // request data on each instrument
    let today = chrono::Utc::now().date_naive();
    let mut date_iter = today;
    for position in portfolio.positions.iter() {
        if let Some(trade) = position.trades.first() {
            date_iter = std::cmp::min(date_iter, trade.date.date());
            histo
                .request(&persistence, &position.instrument, trade.date.date(), today)
                .expect("failed to request data");
        }
    }

    //
    // compute pnl & valuations
    while date_iter < today {
        let date = date_iter.and_hms_opt(23, 59, 00).unwrap();
        date_iter = date_iter.succ_opt().unwrap();

        let portfolio_indicator = PortfolioIndicator::from_portfolio(&portfolio, date);
        let valuations = portfolio_indicator.valuations();
        let pnl = match portfolio_indicator
            .pnl(|instrument, date| histo.get(instrument, date.date()).map(|value| value.close))
        {
            Some(value) => value,
            None => continue,
        };

        println!("{};all;{};{};;", date.format("%Y-%m-%d"), valuations, pnl);
        for (instrument, position_indicator) in portfolio_indicator.positions.iter() {
            let close = histo.get(instrument, date.date()).unwrap().close;
            let valuations = position_indicator.valuations();
            let pnl = position_indicator.pnl(close);
            println!(
                "{};{};{};{};{};{}",
                date.format("%Y-%m-%d"),
                instrument.name,
                valuations,
                pnl,
                position_indicator.unit_price,
                position_indicator.quantity,
            );
        }
    }
}
