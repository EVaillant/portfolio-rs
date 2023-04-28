use crate::error::Error;
use crate::portfolio::Portfolio;
use crate::pricer::PortfolioIndicators;

mod csv;
pub use crate::output::csv::CsvOutput;

pub trait Output {
    fn write_indicators(
        &self,
        portfolio: &Portfolio,
        indicators: &PortfolioIndicators,
    ) -> Result<(), Error>;
}
