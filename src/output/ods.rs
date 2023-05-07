use super::Output;
use crate::error::Error;
use crate::portfolio::Portfolio;
use crate::pricer::PortfolioIndicators;
use spreadsheet_ods::format::{FormatNumberStyle, ValueFormatTrait};
use spreadsheet_ods::{
    currency, percent, Sheet, Value, ValueFormatCurrency, ValueFormatDateTime, ValueFormatRef,
    WorkBook,
};

macro_rules! update_sheet_with_indicator {
    ($sheet:ident, $row:expr, $col:expr, $currency:expr, $indicator:expr) => {
        $sheet.set_value(
            $row as u32,
            $col as u32,
            currency!(&$currency.name, $indicator.valuation),
        );
        $sheet.set_value(
            $row as u32,
            $col + 1 as u32,
            currency!(&$currency.name, $indicator.nominal),
        );
        $sheet.set_value(
            $row as u32,
            $col + 2 as u32,
            currency!(&$currency.name, $indicator.dividends),
        );
        $sheet.set_value(
            $row as u32,
            $col + 3 as u32,
            currency!(&$currency.name, $indicator.tax),
        );
        $sheet.set_value(
            $row as u32,
            $col + 4 as u32,
            percent!($indicator.current_pnl.value_pct),
        );
        $sheet.set_value(
            $row as u32,
            $col + 5 as u32,
            percent!($indicator.daily_pnl.value_pct),
        );
        $sheet.set_value(
            $row as u32,
            $col + 6 as u32,
            percent!($indicator.weekly_pnl.value_pct),
        );
        $sheet.set_value(
            $row as u32,
            $col + 7 as u32,
            percent!($indicator.monthly_pnl.value_pct),
        );
        $sheet.set_value(
            $row as u32,
            $col + 8 as u32,
            percent!($indicator.yearly_pnl.value_pct),
        );
        $sheet.set_value(
            $row as u32,
            $col + 9 as u32,
            currency!(&$currency.name, $indicator.current_pnl.value),
        );
        $sheet.set_value(
            $row as u32,
            $col + 10 as u32,
            currency!(&$currency.name, $indicator.daily_pnl.value),
        );
        $sheet.set_value(
            $row as u32,
            $col + 11 as u32,
            currency!(&$currency.name, $indicator.weekly_pnl.value),
        );
        $sheet.set_value(
            $row as u32,
            $col + 12 as u32,
            currency!(&$currency.name, $indicator.monthly_pnl.value),
        );
        $sheet.set_value(
            $row as u32,
            $col + 13 as u32,
            currency!(&$currency.name, $indicator.yearly_pnl.value),
        );
        $sheet.set_value(
            $row as u32,
            $col + 14 as u32,
            currency!(&$currency.name, $indicator.earning),
        );
        $sheet.set_value(
            $row as u32,
            $col + 15 as u32,
            currency!(&$currency.name, $indicator.earning_latent),
        );
    };
}

pub struct OdsOutput<'a> {
    output_filename: String,
    work_book: WorkBook,
    portfolio: &'a Portfolio,
    indicators: &'a PortfolioIndicators,
}

impl<'a> OdsOutput<'a> {
    pub fn new(
        output_dir: &str,
        portfolio: &'a Portfolio,
        indicators: &'a PortfolioIndicators,
    ) -> Result<Self, Error> {
        let output_filename = format!("{}/{}.ods", output_dir, portfolio.name);
        let path = std::path::Path::new(&output_filename);
        let work_book = if path.exists() {
            spreadsheet_ods::read_ods(path)?
        } else {
            WorkBook::new_empty()
        };
        Ok(Self {
            output_filename,
            work_book,
            portfolio,
            indicators,
        })
    }

    fn add_sheet(&mut self, sheet: Sheet) {
        for i in 0..self.work_book.num_sheets() {
            let i_sheet = self.work_book.sheet(i);
            if i_sheet.name() == sheet.name() {
                self.work_book.remove_sheet(i);
                self.work_book.insert_sheet(i, sheet);
                return;
            }
        }
        self.work_book.push_sheet(sheet);
    }

    fn save(&mut self) -> Result<(), Error> {
        spreadsheet_ods::write_ods(&mut self.work_book, &self.output_filename)?;
        Ok(())
    }

    fn write_position_indicators(&mut self) -> Result<(), Error> {
        let mut sheet = Sheet::new("Indicators");

        // header
        for (i, header_name) in [
            "Date",
            "Cash",
            "Valuation",
            "Nominal",
            "Dividends",
            "Tax",
            "P&L(%)",
            "P&L Daily(%)",
            "P&L Weekly(%)",
            "P&L Monthly(%)",
            "P&L Yearly(%)",
            "P&L",
            "P&L Daily",
            "P&L Weekly",
            "P&L Monthly",
            "P&L Yearly",
            "Earning",
            "Earning + Valuation",
        ]
        .iter()
        .enumerate()
        {
            sheet.set_value(0, i as u32, Value::Text(header_name.to_string()));
        }

        let date_format = self.get_date_format("DD/MM/YYYY")?;
        let date_style = spreadsheet_ods::CellStyle::new("date_style", &date_format);
        let date_style_ref = self.work_book.add_cellstyle(date_style);
        sheet.set_col_cellstyle(0, &date_style_ref);

        let currency_format = self.get_currency_format(&self.portfolio.currency.name)?;
        let currency_style = spreadsheet_ods::CellStyle::new("currency_style", &currency_format);
        let currency_style_ref = self.work_book.add_cellstyle(currency_style);
        for i in [1, 2, 3, 4, 5, 11, 12, 13, 14, 15, 16, 17] {
            sheet.set_col_cellstyle(i, &currency_style_ref);
        }

        for (i, portfolio_indicator) in self.indicators.portfolios.iter().enumerate() {
            sheet.set_value(1 + i as u32, 0, portfolio_indicator.date);
            sheet.set_value(
                1 + i as u32,
                1,
                currency!(&self.portfolio.currency.name, portfolio_indicator.cash),
            );
            update_sheet_with_indicator!(
                sheet,
                1 + i,
                2,
                self.portfolio.currency,
                portfolio_indicator
            );
        }

        self.add_sheet(sheet);
        Ok(())
    }

    fn write_position_instrument_indicators(&mut self, instrument_name: &str) -> Result<(), Error> {
        let mut sheet = Sheet::new(format!("Indicators-{}", instrument_name));

        // header
        for (i, header_name) in [
            "Date",
            "Spot(Close)",
            "Quantity",
            "Unit Price",
            "Valuation",
            "Nominal",
            "Dividends",
            "Tax",
            "P&L(%)",
            "P&L Daily(%)",
            "P&L Weekly(%)",
            "P&L Monthly(%)",
            "P&L Yearly(%)",
            "P&L",
            "P&L Daily",
            "P&L Weekly",
            "P&L Monthly",
            "P&L Yearly",
            "Earning",
            "Earning + Valuation",
        ]
        .iter()
        .enumerate()
        {
            sheet.set_value(0, i as u32, Value::Text(header_name.to_string()));
        }

        let date_format = self.get_date_format("DD/MM/YYYY")?;
        let date_style = spreadsheet_ods::CellStyle::new("date_style", &date_format);
        let date_style_ref = self.work_book.add_cellstyle(date_style);
        sheet.set_col_cellstyle(0, &date_style_ref);

        let currency_format = self.get_currency_format(&self.portfolio.currency.name)?;
        let currency_style = spreadsheet_ods::CellStyle::new("currency_style", &currency_format);
        let currency_style_ref = self.work_book.add_cellstyle(currency_style);
        for i in [1, 3, 4, 5, 6, 7, 13, 14, 15, 16, 17, 18, 19] {
            sheet.set_col_cellstyle(i, &currency_style_ref);
        }

        for (i, position_indicator) in self
            .indicators
            .by_instrument_name(instrument_name)
            .iter()
            .enumerate()
        {
            sheet.set_value(1 + i as u32, 0, position_indicator.date);
            sheet.set_value(1 + i as u32, 1, position_indicator.spot.close());
            sheet.set_value(1 + i as u32, 2, position_indicator.quantity);
            sheet.set_value(
                1 + i as u32,
                3,
                currency!(
                    &position_indicator.instrument.currency.name,
                    position_indicator.unit_price
                ),
            );
            update_sheet_with_indicator!(
                sheet,
                1 + i,
                4,
                position_indicator.instrument,
                position_indicator
            );
        }

        self.add_sheet(sheet);
        Ok(())
    }

    fn get_currency_format(&mut self, name: &str) -> Result<ValueFormatRef, Error> {
        if let Some(value) = self.work_book.currency_format(name) {
            return Ok(value.format_ref());
        }
        if name == "EUR" {
            let mut format_currency = ValueFormatCurrency::new_named(name);
            format_currency
                .part_number()
                .min_integer_digits(1)
                .decimal_places(2)
                .min_decimal_places(2)
                .grouping()
                .build();
            format_currency.part_text(" ").build();
            format_currency.part_currency().symbol("€").build();
            return Ok(self.work_book.add_currency_format(format_currency));
        }
        Err(Error::new_output(format!("unsupported currency {name}")))
    }

    fn get_date_format(&mut self, name: &str) -> Result<ValueFormatRef, Error> {
        if let Some(value) = self.work_book.datetime_format(name) {
            return Ok(value.format_ref());
        }
        if name == "DD/MM/YYYY" {
            let mut v = ValueFormatDateTime::new_named(name);
            v.part_day().style(FormatNumberStyle::Long).build();
            v.part_text("/").build();
            v.part_month().style(FormatNumberStyle::Long).build();
            v.part_text("/").build();
            v.part_year().style(FormatNumberStyle::Long).build();
            return Ok(self.work_book.add_datetime_format(v));
        }
        Err(Error::new_output(format!("unsupported date format {name}")))
    }
}

impl<'a> Output for OdsOutput<'a> {
    fn write_indicators(&mut self) -> Result<(), Error> {
        self.write_position_indicators()?;

        for instrument_name in self.portfolio.get_instrument_name_list() {
            self.write_position_instrument_indicators(instrument_name)?;
        }

        self.save()?;
        Ok(())
    }
}