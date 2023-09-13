use super::Output;
use crate::alias::Date;
use crate::error::Error;
use crate::portfolio::Portfolio;
use crate::pricer::{HeatMapItem, PortfolioIndicators};
use log::debug;
use spreadsheet_ods::format::{FormatNumberStyle, ValueFormatTrait};
use spreadsheet_ods::{
    currency, percent, CellStyleRef, Sheet, Value, ValueFormatCurrency, ValueFormatDateTime,
    ValueFormatRef, WorkBook,
};
use std::collections::{BTreeMap, HashMap};

macro_rules! update_sheet_with_indicator {
    ($sheet:ident, $row:expr, $col:expr, $currency:expr, $indicator:expr) => {
        $sheet.set_value($row, $col, currency!(&$currency.name, $indicator.valuation));
        $sheet.set_value(
            $row,
            $col + 1,
            currency!(&$currency.name, $indicator.nominal),
        );
        $sheet.set_value(
            $row,
            $col + 2,
            currency!(&$currency.name, $indicator.dividends),
        );
        $sheet.set_value($row, $col + 3, currency!(&$currency.name, $indicator.tax));
        $sheet.set_value($row, $col + 4, percent!($indicator.pnl_current.value_pct));
        $sheet.set_value($row, $col + 5, percent!($indicator.pnl_daily.value_pct));
        $sheet.set_value($row, $col + 6, percent!($indicator.pnl_weekly.value_pct));
        $sheet.set_value($row, $col + 7, percent!($indicator.pnl_monthly.value_pct));
        $sheet.set_value($row, $col + 8, percent!($indicator.pnl_yearly.value_pct));
        $sheet.set_value(
            $row,
            $col + 9,
            percent!($indicator.pnl_for_3_months.value_pct),
        );
        $sheet.set_value(
            $row,
            $col + 10,
            percent!($indicator.pnl_for_1_year.value_pct),
        );
        $sheet.set_value(
            $row,
            $col + 11,
            currency!(&$currency.name, $indicator.pnl_current.value),
        );
        $sheet.set_value(
            $row,
            $col + 12,
            currency!(&$currency.name, $indicator.pnl_daily.value),
        );
        $sheet.set_value(
            $row,
            $col + 13,
            currency!(&$currency.name, $indicator.pnl_weekly.value),
        );
        $sheet.set_value(
            $row,
            $col + 14,
            currency!(&$currency.name, $indicator.pnl_monthly.value),
        );
        $sheet.set_value(
            $row,
            $col + 15,
            currency!(&$currency.name, $indicator.pnl_yearly.value),
        );
        $sheet.set_value(
            $row,
            $col + 16,
            currency!(&$currency.name, $indicator.pnl_for_3_months.value),
        );
        $sheet.set_value(
            $row,
            $col + 17,
            currency!(&$currency.name, $indicator.pnl_for_1_year.value),
        );
        $sheet.set_value($row, $col + 18, $indicator.volatility_3_month);
        $sheet.set_value($row, $col + 19, $indicator.volatility_1_year);
        $sheet.set_value(
            $row,
            $col + 20,
            currency!(&$currency.name, $indicator.earning),
        );
        $sheet.set_value(
            $row,
            $col + 21,
            currency!(&$currency.name, $indicator.earning_latent),
        );
    };
}

pub struct OdsOutput<'a> {
    output_filename: String,
    work_book: WorkBook,
    portfolio: &'a Portfolio,
    indicators: &'a PortfolioIndicators,
    filter_indicators: &'a Option<Date>,
}

impl<'a> OdsOutput<'a> {
    pub fn new(
        output_dir: &str,
        portfolio: &'a Portfolio,
        indicators: &'a PortfolioIndicators,
        filter_indicators: &'a Option<Date>,
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
            filter_indicators,
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

    fn remove_sheet(&mut self, name: &str) {
        for i in 0..self.work_book.num_sheets() {
            let i_sheet = self.work_book.sheet(i);
            if i_sheet.name() == name {
                self.work_book.remove_sheet(i);
                return;
            }
        }
    }

    fn save(&mut self) -> Result<(), Error> {
        spreadsheet_ods::write_ods(&mut self.work_book, &self.output_filename)?;
        Ok(())
    }

    fn write_summary(&mut self) -> Result<(), Error> {
        let mut sheet = Sheet::new("Summary");
        // header
        for (i, header_name) in [
            "Instrument",
            "Quantity",
            "Unit Price",
            "Spot (Close)",
            "Valuation",
            "Tax",
            "Nominal",
            "Dividends",
            "P&L",
            "P&L(%)",
            "Distribution",
        ]
        .iter()
        .enumerate()
        {
            sheet.set_value(0, i as u32, Value::Text(header_name.to_string()));
        }

        let currency_style_ref = self.get_currency_style(&self.portfolio.currency.name)?;

        if let Some(portolio) = self.indicators.portfolios.last() {
            let distribution = portolio.make_distribution_global_by_instrument();
            let mut i: u32 = 1;
            for position in portolio.positions.iter() {
                sheet.set_value(i, 0, &position.instrument.name);
                sheet.set_value(i, 1, position.quantity);
                sheet.set_styled_value(
                    i,
                    2,
                    currency!(&position.instrument.currency.name, position.unit_price),
                    &currency_style_ref,
                );
                sheet.set_styled_value(
                    i,
                    3,
                    currency!(&position.instrument.currency.name, position.spot.close()),
                    &currency_style_ref,
                );
                sheet.set_styled_value(
                    i,
                    4,
                    currency!(&position.instrument.currency.name, position.valuation),
                    &currency_style_ref,
                );
                sheet.set_styled_value(
                    i,
                    5,
                    currency!(&position.instrument.currency.name, position.tax),
                    &currency_style_ref,
                );
                sheet.set_styled_value(
                    i,
                    6,
                    currency!(&position.instrument.currency.name, position.nominal),
                    &currency_style_ref,
                );
                sheet.set_styled_value(
                    i,
                    7,
                    currency!(&position.instrument.currency.name, position.dividends),
                    &currency_style_ref,
                );
                sheet.set_styled_value(
                    i,
                    8,
                    currency!(
                        &position.instrument.currency.name,
                        position.pnl_current.value
                    ),
                    &currency_style_ref,
                );
                sheet.set_value(i, 9, percent!(position.pnl_current.value_pct));
                if let Some(instrument_distribution) = distribution.get(&position.instrument.name) {
                    sheet.set_value(i, 10, percent!(*instrument_distribution));
                }
                i += 1;
            }

            i += 1;
            sheet.set_value(i, 0, "Portfolio");
            sheet.set_styled_value(
                i,
                4,
                currency!(&self.portfolio.currency.name, portolio.valuation),
                &currency_style_ref,
            );
            sheet.set_styled_value(
                i,
                5,
                currency!(&self.portfolio.currency.name, portolio.tax),
                &currency_style_ref,
            );
            sheet.set_styled_value(
                i,
                6,
                currency!(&self.portfolio.currency.name, portolio.nominal),
                &currency_style_ref,
            );
            sheet.set_styled_value(
                i,
                7,
                currency!(&self.portfolio.currency.name, portolio.dividends),
                &currency_style_ref,
            );
            sheet.set_styled_value(
                i,
                8,
                currency!(&self.portfolio.currency.name, portolio.pnl_current.value),
                &currency_style_ref,
            );
            sheet.set_value(i, 9, percent!(portolio.pnl_current.value_pct));

            i += 2;
            sheet.set_value(i, 0, "Cash");
            sheet.set_styled_value(
                i,
                1,
                currency!(&self.portfolio.currency.name, portolio.cash),
                &currency_style_ref,
            );
            i += 1;
            sheet.set_value(i, 0, "Cash + Valuation");
            sheet.set_styled_value(
                i,
                1,
                currency!(
                    &self.portfolio.currency.name,
                    portolio.cash + portolio.valuation
                ),
                &currency_style_ref,
            );
            i += 1;
            sheet.set_value(i, 0, "Incoming Transfert");
            sheet.set_styled_value(
                i,
                1,
                currency!(&self.portfolio.currency.name, portolio.incoming_transfer),
                &currency_style_ref,
            );
            i += 1;
            sheet.set_value(i, 0, "Outcoming Transfert");
            sheet.set_styled_value(
                i,
                1,
                currency!(&self.portfolio.currency.name, portolio.outcoming_transfer),
                &currency_style_ref,
            );

            i += 2;
            sheet.set_value(i, 0, "Distribution by Region");
            for (key, value) in portolio.make_distribution_by_region() {
                sheet.set_value(i, 1, Value::Text(key.to_string()));
                sheet.set_value(i, 2, percent!(value));
                i += 1;
            }

            let heat_map = self.indicators.make_month_heat_map();
            i = self.write_month_heat_map_(&mut sheet, "Heat Map By Month", 2 + i, &heat_map)?;

            let heat_map = self.indicators.make_year_heat_map();
            self.write_year_heat_map_(&mut sheet, "Heat Map By Year", 2 + i, &heat_map)?;
        }

        self.add_sheet(sheet);
        Ok(())
    }

    fn write_trades(&mut self) -> Result<(), Error> {
        let mut sheet = Sheet::new("Trades");

        // header
        for (i, header_name) in [
            "Date",
            "Instrument",
            "Quantity",
            "Way",
            "Unit Price",
            "Price",
            "Tax",
        ]
        .iter()
        .enumerate()
        {
            sheet.set_value(0, i as u32, Value::Text(header_name.to_string()));
        }

        let date_style_ref = self.get_date_style("DD/MM/YYYY")?;
        sheet.set_col_cellstyle(0, &date_style_ref);

        let currency_style_ref = self.get_currency_style(&self.portfolio.currency.name)?;
        for i in [4, 5, 6] {
            sheet.set_col_cellstyle(i, &currency_style_ref);
        }

        let mut row = 1;
        for position in self.portfolio.positions.iter() {
            for trade in position.trades.iter() {
                if trade.date.date() > self.indicators.end {
                    continue;
                }
                if trade.date.date() < self.indicators.begin {
                    continue;
                }
                sheet.set_value(row, 0, trade.date);
                sheet.set_value(row, 1, &position.instrument.name);
                sheet.set_value(row, 2, trade.quantity);
                sheet.set_value(row, 3, format!("{}", trade.way));
                sheet.set_value(
                    row,
                    4,
                    currency!(
                        &position.instrument.currency.name,
                        trade.price + trade.tax / trade.quantity
                    ),
                );
                sheet.set_value(
                    row,
                    5,
                    currency!(&position.instrument.currency.name, trade.price),
                );
                sheet.set_value(
                    row,
                    6,
                    currency!(&position.instrument.currency.name, trade.tax),
                );
                row += 1;
            }
        }

        self.add_sheet(sheet);
        Ok(())
    }

    fn write_position_indicators(&mut self) -> Result<(), Error> {
        let mut sheet = Sheet::new("Indicators");

        // header
        for (i, header_name) in [
            "Date",
            "Cash",
            "Incoming Transfert",
            "Outcoming Transfert",
            "Valuation",
            "Nominal",
            "Dividends",
            "Tax",
            "P&L(%)",
            "P&L Daily(%)",
            "P&L Weekly(%)",
            "P&L Monthly(%)",
            "P&L Yearly(%)",
            "P&L for 3 Months(%)",
            "P&L for one Year(%)",
            "P&L",
            "P&L Daily",
            "P&L Weekly",
            "P&L Monthly",
            "P&L Yearly",
            "P&L for 3 Months",
            "P&L for one Year",
            "Volatility 3M",
            "Volatility 1Y",
            "Earning",
            "Earning + Valuation",
        ]
        .iter()
        .enumerate()
        {
            sheet.set_value(0, i as u32, Value::Text(header_name.to_string()));
        }

        let date_style_ref = self.get_date_style("DD/MM/YYYY")?;
        sheet.set_col_cellstyle(0, &date_style_ref);

        let currency_style_ref = self.get_currency_style(&self.portfolio.currency.name)?;
        for i in [1, 2, 3, 4, 5, 6, 7, 15, 16, 17, 18, 19, 20, 21, 24, 25] {
            sheet.set_col_cellstyle(i, &currency_style_ref);
        }

        let mut have_line = false;
        for (i, portfolio_indicator) in self
            .indicators
            .portfolios
            .iter()
            .filter(|item| self.filter_indicators.map_or(true, |date| date < item.date))
            .enumerate()
        {
            have_line = true;
            sheet.set_value(1 + i as u32, 0, portfolio_indicator.date);
            sheet.set_value(
                1 + i as u32,
                1,
                currency!(&self.portfolio.currency.name, portfolio_indicator.cash),
            );
            sheet.set_value(
                1 + i as u32,
                2,
                currency!(
                    &self.portfolio.currency.name,
                    portfolio_indicator.incoming_transfer
                ),
            );
            sheet.set_value(
                1 + i as u32,
                3,
                currency!(
                    &self.portfolio.currency.name,
                    portfolio_indicator.outcoming_transfer
                ),
            );
            update_sheet_with_indicator!(
                sheet,
                1 + i as u32,
                4,
                self.portfolio.currency,
                portfolio_indicator
            );
        }

        if have_line {
            self.add_sheet(sheet);
        } else {
            self.remove_sheet(sheet.name());
        }

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
            "P&L for 3 Months(%)",
            "P&L for Year(%)",
            "P&L",
            "P&L Daily",
            "P&L Weekly",
            "P&L Monthly",
            "P&L Yearly",
            "P&L for 3 Months",
            "P&L for one Year",
            "Volatility 3M",
            "Volatility 1Y",
            "Earning",
            "Earning + Valuation",
        ]
        .iter()
        .enumerate()
        {
            sheet.set_value(0, i as u32, Value::Text(header_name.to_string()));
        }

        let date_style_ref = self.get_date_style("DD/MM/YYYY")?;
        sheet.set_col_cellstyle(0, &date_style_ref);

        let mut defined_currency_col = false;
        let mut have_line = false;
        for (i, position_indicator) in self
            .indicators
            .by_instrument_name(instrument_name)
            .iter()
            .filter(|item| self.filter_indicators.map_or(true, |date| date < item.date))
            .enumerate()
        {
            have_line = true;

            if !defined_currency_col {
                let currency_style_ref =
                    self.get_currency_style(&position_indicator.instrument.currency.name)?;
                for i in [1, 3, 4, 5, 6, 7, 15, 16, 17, 18, 19, 20, 21, 24, 25] {
                    sheet.set_col_cellstyle(i, &currency_style_ref);
                }
                defined_currency_col = true;
            }

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
                1 + i as u32,
                4,
                position_indicator.instrument.currency,
                position_indicator
            );
        }

        if have_line {
            self.add_sheet(sheet);
        } else {
            self.remove_sheet(sheet.name());
        }

        Ok(())
    }

    fn write_heat_map(&mut self) -> Result<(), Error> {
        let mut sheet = Sheet::new("Heat Map");

        let heat_map = self.indicators.make_month_heat_map();
        let mut end_row = self.write_month_heat_map_(&mut sheet, "Portfolio", 0, &heat_map)?;

        for instrument_name in self.portfolio.get_instrument_name_list() {
            let heat_map = self
                .indicators
                .make_month_instrument_heat_map(instrument_name);
            end_row =
                self.write_month_heat_map_(&mut sheet, instrument_name, end_row + 2, &heat_map)?;
        }

        self.add_sheet(sheet);
        Ok(())
    }

    fn write_distribution(&mut self) -> Result<(), Error> {
        let mut sheet = Sheet::new("Distribution");
        if let Some(portfolio) = self.indicators.portfolios.last() {
            let mut row = self.write_distribution_(
                &mut sheet,
                "by region",
                portfolio.make_distribution_by_region(),
                0,
            )?;

            row = self.write_distribution_(
                &mut sheet,
                "by instrument",
                portfolio.make_distribution_global_by_instrument(),
                row + 2,
            )?;

            for region_name in self.portfolio.get_region_name_list() {
                row = self.write_distribution_(
                    &mut sheet,
                    &format!("by instrument in {}", region_name),
                    portfolio.make_distribution_by_instrument(region_name),
                    row + 2,
                )?;
            }
        }
        self.add_sheet(sheet);
        Ok(())
    }

    fn write_distribution_(
        &mut self,
        sheet: &mut Sheet,
        name: &str,
        data: HashMap<String, f64>,
        mut row: u32,
    ) -> Result<u32, Error> {
        sheet.set_value(row, 0, Value::Text(name.to_string()));
        for (key, value) in data {
            sheet.set_value(row, 1, Value::Text(key.to_string()));
            sheet.set_value(row, 2, percent!(value));
            row += 1;
        }
        Ok(row)
    }

    fn write_month_heat_map_(
        &mut self,
        sheet: &mut Sheet,
        name: &str,
        mut row: u32,
        heat_map: &BTreeMap<i32, HeatMapItem>,
    ) -> Result<u32, Error> {
        sheet.set_value(row, 0, Value::Text(name.to_string()));
        for (i, header_name) in [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct,", "Nov", "Dec",
        ]
        .iter()
        .enumerate()
        {
            sheet.set_value(row, i as u32 + 2, Value::Text(header_name.to_string()));
        }
        row += 1;

        for (year, item) in heat_map {
            sheet.set_value(row, 1, year);
            for (pos, value) in item.data().iter().enumerate() {
                if let Some(pct) = value {
                    sheet.set_value(row, 2 + pos as u32, percent!(*pct));
                }
            }
            row += 1;
        }

        Ok(row)
    }

    fn write_year_heat_map_(
        &mut self,
        sheet: &mut Sheet,
        name: &str,
        mut row: u32,
        heat_map: &BTreeMap<i32, f64>,
    ) -> Result<u32, Error> {
        sheet.set_value(row, 0, Value::Text(name.to_string()));
        for (year, value) in heat_map {
            sheet.set_value(row, 1, year);
            sheet.set_value(row, 2, percent!(*value));
            row += 1;
        }
        Ok(row)
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

    fn get_date_style(&mut self, date_format: &str) -> Result<CellStyleRef, Error> {
        let style_name = format!("date_style_{}", date_format);
        if let Some(value) = self.work_book.cellstyle(&style_name) {
            return Ok(value.style_ref());
        }

        let value_format_ref = self.get_date_format(date_format)?;
        let date_style = spreadsheet_ods::CellStyle::new(&style_name, &value_format_ref);
        let date_style_ref = self.work_book.add_cellstyle(date_style);
        Ok(date_style_ref)
    }

    fn get_currency_style(&mut self, currency_name: &str) -> Result<CellStyleRef, Error> {
        let style_name = format!("currency_style_{}", currency_name);
        if let Some(value) = self.work_book.cellstyle(&style_name) {
            return Ok(value.style_ref());
        }

        let value_format_ref = self.get_currency_format(currency_name)?;
        let currency_style = spreadsheet_ods::CellStyle::new(&style_name, &value_format_ref);
        let currency_style_ref = self.work_book.add_cellstyle(currency_style);
        Ok(currency_style_ref)
    }
}

impl<'a> Output for OdsOutput<'a> {
    fn write_indicators(&mut self) -> Result<(), Error> {
        debug!("write summary");
        self.write_summary()?;

        debug!("write trades");
        self.write_trades()?;

        debug!("write heat map");
        self.write_heat_map()?;

        debug!("write distribution");
        self.write_distribution()?;

        debug!("write position indicators");
        self.write_position_indicators()?;

        for instrument_name in self.portfolio.get_instrument_name_list() {
            debug!("write position indicators for {}", instrument_name);
            self.write_position_instrument_indicators(instrument_name)?;
        }

        debug!("save");
        self.save()?;
        Ok(())
    }
}
