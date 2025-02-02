#[derive(Debug)]
pub enum Error {
    Historical(String),
    Portfolio(String),
    Referential(String),
    Output(String),
    Io(std::io::Error),
    Ods(spreadsheet_ods::OdsError),
    Rusqlite(rusqlite::Error),
    SerdeJson(serde_json::Error),
    YahooFinance(yahoo_finance_api::Error),
}

impl Error {
    pub fn new_historical<T: Into<String>>(msg: T) -> Error {
        Error::Historical(msg.into())
    }

    pub fn new_portfolio<T: Into<String>>(msg: T) -> Error {
        Error::Portfolio(msg.into())
    }

    pub fn new_referential<T: Into<String>>(msg: T) -> Error {
        Error::Referential(msg.into())
    }

    pub fn new_output<T: Into<String>>(msg: T) -> Error {
        Error::Output(msg.into())
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Error::Rusqlite(error)
    }
}

impl From<spreadsheet_ods::OdsError> for Error {
    fn from(error: spreadsheet_ods::OdsError) -> Self {
        Error::Ods(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::SerdeJson(error)
    }
}

impl From<yahoo_finance_api::Error> for Error {
    fn from(error: yahoo_finance_api::Error) -> Self {
        Error::YahooFinance(error)
    }
}
