use crate::error::Error;

mod csv;
mod ods;
mod ods_helper;

pub use self::csv::CsvOutput;
pub use self::ods::OdsOutput;

pub trait Output {
    fn write_indicators(&mut self) -> Result<(), Error>;
}
