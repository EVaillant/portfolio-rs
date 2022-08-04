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
mod referential;

use historical::{HistoricalData, YahooProvider};
use persistence::SQLitePersistance;
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

    info!("request instrument historic data");
    let today = chrono::Utc::now().date();
    for position in portfolio.positions.iter() {
        if let Some(trade) = position.trades.get(0) {
            histo
                .request(
                    &persistence,
                    &position.instrument,
                    trade.date.date(),
                    today.succ(),
                )
                .expect("failed to request data");
        }
    }
}
