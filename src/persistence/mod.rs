use crate::alias::Date;
use crate::error::{Error, ErrorKind};
use crate::historical::DataFrame;
use crate::marketdata::Instrument;
use rusqlite::{Connection, Result};

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Error::new(ErrorKind::Persistance, format!("sqlite error : {}", error))
    }
}

struct SQLiteDate(Date);
impl rusqlite::types::FromSql for SQLiteDate {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value {
            rusqlite::types::ValueRef::Text(txt) => {
                let str_txt = std::str::from_utf8(txt)
                    .map_err(|_| rusqlite::types::FromSqlError::InvalidType)?;
                let naive_date = chrono::NaiveDate::parse_from_str(str_txt, "%Y-%m-%d");
                match naive_date {
                    Ok(value) => Ok(SQLiteDate(value)),
                    Err(_) => Err(rusqlite::types::FromSqlError::InvalidType),
                }
            }
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

pub struct SQLitePersistance {
    connection: Connection,
}

impl SQLitePersistance {
    pub fn new(file: &str) -> Result<Self, Error> {
        let connection = Connection::open(file)?;
        let instance = Self { connection };
        instance.setup()?;
        Ok(instance)
    }

    fn setup(&self) -> Result<(), Error> {
        self.connection.execute(
          "CREATE TABLE IF NOT EXISTS Historical (instrument TEXT, date TEXT, open REAL, close REAL, high REAL, low REAL, PRIMARY KEY(\"instrument\",\"date\"))",
          (),
        )?;
        Ok(())
    }
}

impl crate::historical::Persistance for SQLitePersistance {
    fn save<P>(&self, instrument: &Instrument, datas: &[P]) -> Result<(), Error>
    where
        P: DataFrame,
    {
        self.connection.execute_batch("BEGIN TRANSACTION;")?;
        let mut stmt = self.connection.prepare(
          "INSERT OR REPLACE INTO Historical (instrument, date, open, close, high, low) VALUES(?, ?, ?, ?, ?, ?)",
        )?;

        for data in datas.iter() {
            stmt.execute((
                &instrument.name,
                data.date().format("%Y-%m-%d").to_string(),
                data.open(),
                data.close(),
                data.high(),
                data.low(),
            ))?;
        }

        self.connection.execute_batch("COMMIT TRANSACTION;")?;
        Ok(())
    }

    fn load<P>(&self, instrument: &Instrument) -> Result<Option<(Date, Date, Vec<P>)>, Error>
    where
        P: DataFrame,
    {
        let mut stmt = self
            .connection
            .prepare("SELECT * FROM Historical WHERE instrument = ?")?;

        let rows = stmt.query_map((&instrument.name,), |row| {
            Ok(P::new(
                row.get::<usize, SQLiteDate>(1)?.0,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })?;

        let mut datas = Vec::new();
        for item in rows {
            datas.push(item?);
        }
        datas.sort_by(|left, right| left.date().cmp(right.date()));
        let first = datas.first();
        let last = datas.last();

        match (first, last) {
            (Some(value1), Some(value2)) => Ok(Some((*value1.date(), *value2.date(), datas))),
            (_, _) => Ok(None),
        }
    }
}
