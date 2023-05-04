use crate::error::Error;

mod csv;
pub use crate::output::csv::CsvOutput;

mod ods;
pub use crate::output::ods::OdsOutput;

pub trait Output {
    fn write_indicators(&mut self) -> Result<(), Error>;
}
