use crate::error::Error;

mod csv;
mod ods;
mod ods_helper;
mod portfolio_performance;

pub use self::csv::CsvOutput;
pub use self::ods::OdsOutput;
pub use self::portfolio_performance::PortfolioPerformanceOutput;

pub trait Output {
    fn write(&mut self) -> Result<(), Error>;
}
