use super::Output;
use super::ods_helper::{TableBuilder, TableBuilderStyleResolver};
use crate::alias::Date;
use crate::error::Error;
use crate::marketdata::Instrument;
use crate::portfolio::{Portfolio, Trade};
use crate::pricer::{
    ClosePositionIndicator, HeatMap, HeatMapComputeMode, HeatMapPeriod, InstrumentIndicator,
    PortfolioIndicator, PortfolioIndicators, PositionIndicator, PositionIndicators,
    RegionIndicator, RegionIndicatorInstrument,
};
use chrono::Datelike;
use log::debug;
use spreadsheet_ods::format::{FormatNumberStyle, ValueFormatTrait};
use spreadsheet_ods::{
    CellStyleRef, Sheet, Value, ValueFormatCurrency, ValueFormatDateTime, ValueFormatRef, WorkBook,
    currency, percent,
};

use std::collections::BTreeMap;
use std::rc::Rc;

pub struct OdsOutput<'a> {
    output_filename: String,
    work_book: WorkBook,
    portfolio: &'a Portfolio,
    indicators: &'a PortfolioIndicators,
    filter_indicators: &'a Option<Date>,
    details_sheet: bool,
}

impl TableBuilderStyleResolver for OdsOutput<'_> {
    fn get_style(&self, _header: &str, value: &Value) -> Option<CellStyleRef> {
        match value {
            Value::Currency(_, currency_name) => self.get_currency_style(currency_name),
            Value::DateTime(_) => self.get_date_style("DD/MM/YYYY"),
            _ => None,
        }
    }
}

impl<'a> OdsOutput<'a> {
    pub fn new(
        output_dir: &str,
        portfolio: &'a Portfolio,
        indicators: &'a PortfolioIndicators,
        filter_indicators: &'a Option<Date>,
        details_sheet: bool,
    ) -> Result<Self, Error> {
        let output_filename = format!("{}/{}.ods", output_dir, portfolio.name);
        Ok(Self {
            output_filename,
            work_book: WorkBook::new_empty(),
            portfolio,
            indicators,
            filter_indicators,
            details_sheet,
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

    fn create_style(&mut self) -> Result<(), Error> {
        self.create_date_style("DD/MM/YYYY")?;
        self.create_currency_style(&self.portfolio.currency.name)?;
        for instrument in self
            .portfolio
            .positions
            .iter()
            .map(|position| &position.instrument)
        {
            self.create_currency_style(&instrument.currency.name)?;
        }
        Ok(())
    }

    fn write_summary(&mut self) {
        let mut sheet = Sheet::new("Summary");

        if let Some(portfolio) = self.indicators.portfolios.last() {
            let intrument_indicators = InstrumentIndicator::from_portfolio(portfolio);
            let inputs = portfolio
                .positions
                .iter()
                .filter(|position| !position.is_close);

            sheet.set_value(0, 0, "Open Position");
            let mut row = TableBuilder::new()
                .add("Instrument Description", |position: &&PositionIndicator| {
                    &position.instrument.description
                })
                .add("Quantity", |position: &&PositionIndicator| {
                    position.quantity
                })
                .add("Unit Price", |position: &&PositionIndicator| {
                    currency!(&position.instrument.currency.name, position.unit_price)
                })
                .add("Spot (Close)", |position: &&PositionIndicator| {
                    currency!(&position.instrument.currency.name, position.spot.close)
                })
                .add("Spot (Date)", |position: &&PositionIndicator| {
                    position.spot.date
                })
                .add("Valuation", |position: &&PositionIndicator| {
                    currency!(&position.instrument.currency.name, position.valuation)
                })
                .add("Fees", |position: &&PositionIndicator| {
                    currency!(&position.instrument.currency.name, position.fees)
                })
                .add("Nominal", |position: &&PositionIndicator| {
                    currency!(&position.instrument.currency.name, position.nominal)
                })
                .add("Dividends", |position: &&PositionIndicator| {
                    currency!(&position.instrument.currency.name, position.dividends)
                })
                .add("TWR", |position: &&PositionIndicator| {
                    percent!(position.twr)
                })
                .add("P&L", |position: &&PositionIndicator| {
                    currency!(&position.instrument.currency.name, position.pnl_currency)
                })
                .add("P&L(%)", |position: &&PositionIndicator| {
                    percent!(position.pnl_percent)
                })
                .add("P&L Volatility (3M)", |position: &&PositionIndicator| {
                    percent!(position.pnl_volatility_3m)
                })
                .add_optional("IRR", |position: &&PositionIndicator| {
                    position.irr.map(|value| percent!(value))
                })
                .add_optional("Distribution", |position: &&PositionIndicator| {
                    intrument_indicators
                        .iter()
                        .find(|indicator| indicator.instrument == position.instrument)
                        .map(|item| percent!(item.valuation_percent))
                })
                .write(&mut sheet, self, 0, 1, inputs);

            TableBuilder::new()
                .add("", |portfolio: &&PortfolioIndicator| {
                    currency!(&self.portfolio.currency.name, portfolio.open_valuation)
                })
                .add("", |portfolio: &&PortfolioIndicator| {
                    currency!(&self.portfolio.currency.name, portfolio.open_fees)
                })
                .add("", |portfolio: &&PortfolioIndicator| {
                    currency!(&self.portfolio.currency.name, portfolio.open_nominal)
                })
                .add("", |portfolio: &&PortfolioIndicator| {
                    currency!(&self.portfolio.currency.name, portfolio.open_dividends)
                })
                .add("", |portfolio: &&PortfolioIndicator| {
                    percent!(portfolio.open_twr)
                })
                .add("", |portfolio: &&PortfolioIndicator| {
                    currency!(&self.portfolio.currency.name, portfolio.open_pnl_currency)
                })
                .add("", |portfolio: &&PortfolioIndicator| {
                    percent!(portfolio.open_pnl_percent)
                })
                .add("", |portfolio: &&PortfolioIndicator| {
                    percent!(portfolio.open_pnl_volatility_3m)
                })
                .write_line(&mut sheet, self, row + 1, 6, &portfolio);
            row += 3;

            sheet.set_value(row, 0, "Porfolio");
            TableBuilder::new()
                .add("Cash", |portfolio: &&PortfolioIndicator| {
                    currency!(&self.portfolio.currency.name, portfolio.cash)
                })
                .add("Valuation", |portfolio: &&PortfolioIndicator| {
                    currency!(&self.portfolio.currency.name, portfolio.valuation)
                })
                .add("P&L(%)", |portfolio: &&PortfolioIndicator| {
                    percent!(portfolio.pnl_percent)
                })
                .add("P&L", |portfolio: &&PortfolioIndicator| {
                    currency!(&self.portfolio.currency.name, portfolio.pnl_currency)
                })
                .add("P&L Volatility (3M)", |portfolio: &&PortfolioIndicator| {
                    percent!(portfolio.pnl_volatility_3m)
                })
                .add("TWR", |portfolio: &&PortfolioIndicator| {
                    percent!(portfolio.twr)
                })
                .add("Fees", |portfolio: &&PortfolioIndicator| {
                    currency!(&self.portfolio.currency.name, portfolio.fees)
                })
                .add("Fees (%)", |portfolio: &&PortfolioIndicator| {
                    percent!(portfolio.fees_percent)
                })
                .add("Incoming Transfert", |portfolio: &&PortfolioIndicator| {
                    currency!(&self.portfolio.currency.name, portfolio.incoming_transfer)
                })
                .add("Outcoming Transfert", |portfolio: &&PortfolioIndicator| {
                    currency!(&self.portfolio.currency.name, portfolio.outcoming_transfer)
                })
                .write_reversed(&mut sheet, self, row, 1, std::iter::once(portfolio));
            row += 11;

            let close_position_row = self.write_close_positions_(&mut sheet, row, 1, Some(5));
            if close_position_row != 0 {
                sheet.set_value(row, 0, "Close Position");
                row = close_position_row + 1;
            }

            let region_indicators = RegionIndicator::from_portfolio(portfolio);
            row = self.write_distribution_by_region(
                &mut sheet,
                "Distribution by Region",
                &region_indicators,
                row,
            );

            let heat_map = HeatMap::from_portfolios(
                self.indicators,
                HeatMapPeriod::Monthly,
                HeatMapComputeMode::Delta,
                |indicator| indicator.pnl_percent,
            );
            row = self.write_heat_map_monthly_(&mut sheet, "P&L By Month", row + 1, heat_map);

            let heat_map = HeatMap::from_portfolios(
                self.indicators,
                HeatMapPeriod::Yearly,
                HeatMapComputeMode::Delta,
                |indicator| indicator.pnl_percent,
            );
            row = self.write_heat_map_yearly_percent_(&mut sheet, "P&L By Year", row + 2, heat_map);

            let heat_map = HeatMap::from_portfolios(
                self.indicators,
                HeatMapPeriod::Yearly,
                HeatMapComputeMode::Value,
                |indicator| indicator.incoming_transfer,
            );
            self.write_heat_map_yearly_currency_(
                &mut sheet,
                "Incoming Transfert By Year",
                row + 2,
                heat_map,
                &self.portfolio.currency.name,
            );
        }

        self.add_sheet(sheet);
    }

    fn write_trades(&mut self) {
        let inputs = self.portfolio.positions.iter().flat_map(|position| {
            position
                .trades
                .iter()
                .filter(|trade| {
                    (trade.date.date() <= self.indicators.end)
                        && (trade.date.date() >= self.indicators.begin)
                        && self
                            .filter_indicators
                            .is_none_or(|date| date < trade.date.date())
                })
                .map(|trade| (&position.instrument, trade))
        });

        let mut table = TableBuilder::new();
        table
            .add("Date", |(_, trade): &(&Rc<Instrument>, &Trade)| trade.date)
            .add(
                "Instrument",
                |(instrument, _): &(&Rc<Instrument>, &Trade)| &instrument.name,
            )
            .add("Quantity", |(_, trade): &(&Rc<Instrument>, &Trade)| {
                trade.quantity
            })
            .add("Way", |(_, trade): &(&Rc<Instrument>, &Trade)| {
                trade.way.to_string()
            })
            .add(
                "Unit Price",
                |(instrument, trade): &(&Rc<Instrument>, &Trade)| {
                    currency!(
                        &instrument.currency.name,
                        trade.price + trade.fees / trade.quantity
                    )
                },
            )
            .add(
                "Price",
                |(instrument, trade): &(&Rc<Instrument>, &Trade)| {
                    currency!(&instrument.currency.name, trade.price)
                },
            )
            .add("Fees", |(instrument, trade): &(&Rc<Instrument>, &Trade)| {
                currency!(&instrument.currency.name, trade.fees)
            });

        let mut sheet = Sheet::new("Trades");
        table.write(&mut sheet, self, 0, 0, inputs);
        self.add_sheet(sheet);
    }

    fn write_close_positions(&mut self) {
        let mut sheet = Sheet::new("Close Position");
        if self.write_close_positions_(&mut sheet, 0, 0, None) != 0 {
            self.add_sheet(sheet);
        } else {
            self.remove_sheet(sheet.name());
        }
    }

    fn write_close_positions_(
        &mut self,
        sheet: &mut Sheet,
        row: u32,
        col: u32,
        limit_size: Option<usize>,
    ) -> u32 {
        let mut inputs = ClosePositionIndicator::from_portfolios(&self.indicators.portfolios);
        inputs.sort_by(|left, right| right.close.cmp(&left.close));
        if inputs.is_empty() {
            return 0;
        }

        let mut table = TableBuilder::new();
        table
            .add(
                "Instrument Description",
                |position: &&ClosePositionIndicator| &position.instrument.description,
            )
            .add("Open", |position: &&ClosePositionIndicator| position.open)
            .add("Close", |position: &&ClosePositionIndicator| position.close)
            .add("P&L", |position: &&ClosePositionIndicator| {
                currency!(&position.instrument.currency.name, position.pnl_currency)
            })
            .add("Fees", |position: &&ClosePositionIndicator| {
                currency!(&position.instrument.currency.name, position.fees)
            })
            .add("Dividends", |position: &&ClosePositionIndicator| {
                currency!(&position.instrument.currency.name, position.dividends)
            })
            .add("TWR", |position: &&ClosePositionIndicator| {
                percent!(position.twr)
            })
            .add_optional("IRR", |position: &&ClosePositionIndicator| {
                position.irr.map(|value| percent!(value))
            });

        if let Some(size) = limit_size {
            table.write(sheet, self, row, col, inputs.iter().take(size))
        } else {
            table.write(sheet, self, row, col, inputs.iter())
        }
    }

    fn write_position_indicators(&mut self) {
        let inputs = self
            .indicators
            .portfolios
            .iter()
            .take_while(|item| self.filter_indicators.is_none_or(|date| date < item.date));

        let mut table = TableBuilder::new();
        table
            .add("Date", |portfolio_indicator: &&PortfolioIndicator| {
                portfolio_indicator.date
            })
            .add("Valuation", |portfolio_indicator: &&PortfolioIndicator| {
                currency!(&self.portfolio.currency.name, portfolio_indicator.valuation)
            })
            .add("Nominal", |portfolio_indicator: &&PortfolioIndicator| {
                currency!(&self.portfolio.currency.name, portfolio_indicator.nominal)
            })
            .add(
                "Incoming Transfert",
                |portfolio_indicator: &&PortfolioIndicator| {
                    currency!(
                        &self.portfolio.currency.name,
                        portfolio_indicator.incoming_transfer
                    )
                },
            )
            .add(
                "Outcoming Transfert",
                |portfolio_indicator: &&PortfolioIndicator| {
                    currency!(
                        &self.portfolio.currency.name,
                        portfolio_indicator.outcoming_transfer
                    )
                },
            )
            .add("Cash", |portfolio_indicator: &&PortfolioIndicator| {
                currency!(&self.portfolio.currency.name, portfolio_indicator.cash)
            })
            .add("Dividends", |portfolio_indicator: &&PortfolioIndicator| {
                currency!(&self.portfolio.currency.name, portfolio_indicator.dividends)
            })
            .add("Fees", |portfolio_indicator: &&PortfolioIndicator| {
                currency!(&self.portfolio.currency.name, portfolio_indicator.fees)
            })
            .add("P&L", |portfolio_indicator: &&PortfolioIndicator| {
                currency!(
                    &self.portfolio.currency.name,
                    portfolio_indicator.pnl_currency
                )
            })
            .add("P&L(%)", |portfolio_indicator: &&PortfolioIndicator| {
                percent!(portfolio_indicator.pnl_percent)
            })
            .add(
                "P&L Volatility (3M)",
                |portfolio_indicator: &&PortfolioIndicator| {
                    percent!(portfolio_indicator.pnl_volatility_3m)
                },
            )
            .add("TWR", |portfolio_indicator: &&PortfolioIndicator| {
                percent!(portfolio_indicator.twr)
            })
            .add("Earning", |portfolio_indicator: &&PortfolioIndicator| {
                currency!(&self.portfolio.currency.name, portfolio_indicator.earning)
            })
            .add(
                "Earning Latent",
                |portfolio_indicator: &&PortfolioIndicator| {
                    currency!(
                        &self.portfolio.currency.name,
                        portfolio_indicator.earning_latent
                    )
                },
            );

        let mut sheet = Sheet::new("Indicators");
        if table.write(&mut sheet, self, 0, 0, inputs) != 1 {
            self.add_sheet(sheet);
        } else {
            self.remove_sheet(sheet.name());
        }
    }

    fn write_position_instrument_indicators(&mut self, indicators: PositionIndicators) {
        let mut is_close = false;
        let inputs = indicators.positions.iter().take_while(|item| {
            if !is_close {
                is_close = item.is_close;
                self.filter_indicators.is_none_or(|date| date < item.date)
            } else {
                !is_close
            }
        });

        let mut table = TableBuilder::new();
        table
            .add("Date", |position_indicator: &&&PositionIndicator| {
                position_indicator.date
            })
            .add("Spot(Close)", |position_indicator: &&&PositionIndicator| {
                currency!(
                    &position_indicator.instrument.currency.name,
                    position_indicator.spot.close
                )
            })
            .add("Quantity", |position_indicator: &&&PositionIndicator| {
                position_indicator.quantity
            })
            .add("Unit Price", |position_indicator: &&&PositionIndicator| {
                currency!(
                    &position_indicator.instrument.currency.name,
                    position_indicator.unit_price
                )
            })
            .add("Valuation", |position_indicator: &&&PositionIndicator| {
                currency!(
                    &position_indicator.instrument.currency.name,
                    position_indicator.valuation
                )
            })
            .add("Nominal", |position_indicator: &&&PositionIndicator| {
                currency!(
                    &position_indicator.instrument.currency.name,
                    position_indicator.nominal
                )
            })
            .add("Cashflow", |position_indicator: &&&PositionIndicator| {
                currency!(
                    &position_indicator.instrument.currency.name,
                    position_indicator.cashflow
                )
            })
            .add("Dividends", |position_indicator: &&&PositionIndicator| {
                currency!(
                    &position_indicator.instrument.currency.name,
                    position_indicator.dividends
                )
            })
            .add("Fees", |position_indicator: &&&PositionIndicator| {
                currency!(
                    &position_indicator.instrument.currency.name,
                    position_indicator.fees
                )
            })
            .add("P&L", |position_indicator: &&&PositionIndicator| {
                currency!(
                    &position_indicator.instrument.currency.name,
                    position_indicator.pnl_currency
                )
            })
            .add("P&L(%)", |position_indicator: &&&PositionIndicator| {
                percent!(position_indicator.pnl_percent)
            })
            .add(
                "P&L Volatility (3M)",
                |position_indicator: &&&PositionIndicator| {
                    percent!(position_indicator.pnl_volatility_3m)
                },
            )
            .add("TWR", |position_indicator: &&&PositionIndicator| {
                percent!(position_indicator.twr)
            })
            .add_optional("IRR", |position: &&&PositionIndicator| {
                position.irr.map(|value| percent!(value))
            })
            .add("Earning", |position_indicator: &&&PositionIndicator| {
                currency!(
                    &position_indicator.instrument.currency.name,
                    position_indicator.earning
                )
            })
            .add(
                "Earning Latent",
                |position_indicator: &&&PositionIndicator| {
                    currency!(
                        &position_indicator.instrument.currency.name,
                        position_indicator.earning_latent
                    )
                },
            )
            .add("Is Close", |position_indicator: &&&PositionIndicator| {
                Value::Boolean(position_indicator.is_close)
            });

        let mut sheet = Sheet::new(format!(
            "Indicators-{}-{}",
            indicators.instrument_name, indicators.position_index
        ));
        if table.write(&mut sheet, self, 0, 0, inputs) != 1 {
            self.add_sheet(sheet);
        } else {
            self.remove_sheet(sheet.name());
        }
    }

    fn write_heat_map(&mut self) {
        let mut sheet = Sheet::new("Heat Map");

        let heat_map = HeatMap::from_portfolios(
            self.indicators,
            HeatMapPeriod::Monthly,
            HeatMapComputeMode::Delta,
            |indicator| indicator.pnl_percent,
        );
        let mut row = self.write_heat_map_monthly_(&mut sheet, "Portfolio Monthly", 0, heat_map);
        let heat_map = HeatMap::from_portfolios(
            self.indicators,
            HeatMapPeriod::Yearly,
            HeatMapComputeMode::Delta,
            |indicator| indicator.pnl_percent,
        );
        row =
            self.write_heat_map_yearly_percent_(&mut sheet, "Portfolio Yearly", row + 1, heat_map);

        for instrument_name in self.portfolio.get_instrument_name_list() {
            for position_index in self.indicators.get_position_index_list(instrument_name) {
                let position_indicators = self
                    .indicators
                    .get_position_indicators(instrument_name, position_index);

                let heat_map = HeatMap::from_positions(
                    &position_indicators,
                    HeatMapPeriod::Monthly,
                    HeatMapComputeMode::Delta,
                    |indicator| indicator.pnl_percent,
                );
                row = self.write_heat_map_monthly_(
                    &mut sheet,
                    &format!("Portfolio Monthly {} / {}", instrument_name, position_index),
                    row + 1,
                    heat_map,
                );

                let heat_map = HeatMap::from_positions(
                    &position_indicators,
                    HeatMapPeriod::Yearly,
                    HeatMapComputeMode::Delta,
                    |indicator| indicator.pnl_percent,
                );
                row = self.write_heat_map_yearly_percent_(
                    &mut sheet,
                    &format!("Portfolio Yearly {} / {}", instrument_name, position_index),
                    row + 1,
                    heat_map,
                );
            }
        }

        self.add_sheet(sheet);
    }

    fn write_distribution(&mut self) {
        let mut sheet = Sheet::new("Distribution");
        if let Some(portfolio) = self.indicators.portfolios.last() {
            let region_indicators = RegionIndicator::from_portfolio(portfolio);
            let mut row =
                self.write_distribution_by_region(&mut sheet, "by region", &region_indicators, 0);

            let intrument_indicators = InstrumentIndicator::from_portfolio(portfolio);
            row = self.write_distribution_by_instrument(
                &mut sheet,
                "by instrument",
                &intrument_indicators,
                row + 2,
            );

            for region_indicator in region_indicators {
                row = self.write_distribution_global_by_instrument(
                    &mut sheet,
                    &format!("by instrument in {}", region_indicator.region_name),
                    &region_indicator.instruments,
                    row + 2,
                );
            }
        }
        self.add_sheet(sheet);
    }

    fn write_distribution_by_region(
        &mut self,
        sheet: &mut Sheet,
        name: &str,
        data: &Vec<RegionIndicator>,
        mut row: u32,
    ) -> u32 {
        sheet.set_value(row, 0, Value::Text(name.to_string()));
        for indicator in data {
            sheet.set_value(row, 1, Value::Text(indicator.region_name.to_string()));
            sheet.set_value(row, 2, percent!(indicator.valuation_percent));
            row += 1;
        }
        row
    }

    fn write_distribution_by_instrument(
        &mut self,
        sheet: &mut Sheet,
        name: &str,
        data: &Vec<InstrumentIndicator>,
        mut row: u32,
    ) -> u32 {
        sheet.set_value(row, 0, Value::Text(name.to_string()));
        for indicator in data {
            sheet.set_value(row, 1, Value::Text(indicator.instrument.name.to_string()));
            sheet.set_value(row, 2, percent!(indicator.valuation_percent));
            row += 1;
        }
        row
    }

    fn write_distribution_global_by_instrument(
        &mut self,
        sheet: &mut Sheet,
        name: &str,
        data: &Vec<RegionIndicatorInstrument>,
        mut row: u32,
    ) -> u32 {
        sheet.set_value(row, 0, Value::Text(name.to_string()));
        for indicator in data {
            sheet.set_value(row, 1, Value::Text(indicator.instrument.name.to_string()));
            sheet.set_value(row, 2, percent!(indicator.valuation_percent));
            row += 1;
        }
        row
    }

    fn write_heat_map_monthly_(
        &mut self,
        sheet: &mut Sheet,
        name: &str,
        mut row: u32,
        heat_map: HeatMap,
    ) -> u32 {
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

        let mut data: BTreeMap<i32, [Option<f64>; 12]> = Default::default();
        for (date, value) in heat_map.data {
            let row = data.entry(date.year()).or_default();
            row[date.month0() as usize] = Some(value);
        }

        for (year, values) in data {
            sheet.set_value(row, 1, year);
            for (pos, value) in values.into_iter().enumerate() {
                if let Some(pct) = value {
                    sheet.set_value(row, 2 + pos as u32, percent!(pct));
                }
            }
            row += 1;
        }

        row
    }

    fn write_heat_map_yearly_percent_(
        &mut self,
        sheet: &mut Sheet,
        name: &str,
        mut row: u32,
        heat_map: HeatMap,
    ) -> u32 {
        sheet.set_value(row, 0, Value::Text(name.to_string()));
        for (date, value) in heat_map.data {
            sheet.set_value(row, 1, date.year());
            sheet.set_value(row, 2, percent!(value));
            row += 1;
        }
        row
    }

    fn write_heat_map_yearly_currency_(
        &mut self,
        sheet: &mut Sheet,
        name: &str,
        mut row: u32,
        heat_map: HeatMap,
        currency_name: &str,
    ) -> u32 {
        let currency_style = self.get_currency_style(currency_name);
        sheet.set_value(row, 0, Value::Text(name.to_string()));
        for (date, value) in heat_map.data {
            sheet.set_value(row, 1, date.year());
            if let Some(style) = &currency_style {
                sheet.set_styled_value(row, 2, currency!(currency_name, value), style);
            } else {
                sheet.set_value(row, 2, currency!(currency_name, value));
            }
            row += 1;
        }
        row
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

    fn create_date_style(&mut self, date_format: &str) -> Result<(), Error> {
        if self.get_date_style(date_format).is_some() {
            return Ok(());
        }

        let value_format_ref = self.get_date_format(date_format)?;
        let date_style = spreadsheet_ods::CellStyle::new(
            Self::make_date_style_name_(date_format),
            &value_format_ref,
        );
        self.work_book.add_cellstyle(date_style);
        Ok(())
    }

    fn create_currency_style(&mut self, currency_name: &str) -> Result<(), Error> {
        if self.get_currency_style(currency_name).is_some() {
            return Ok(());
        }

        let value_format_ref = self.get_currency_format(currency_name)?;
        let currency_style = spreadsheet_ods::CellStyle::new(
            Self::make_currency_style_name_(currency_name),
            &value_format_ref,
        );
        self.work_book.add_cellstyle(currency_style);
        Ok(())
    }

    fn make_currency_style_name_(currency_name: &str) -> String {
        format!("currency_style_{}", currency_name)
    }

    fn make_date_style_name_(date_format: &str) -> String {
        format!("date_style_{}", date_format)
    }

    fn get_style_by_name_(&self, name: &str) -> Option<CellStyleRef> {
        self.work_book.cellstyle(name).map(|item| item.style_ref())
    }

    fn get_currency_style(&self, currency_name: &str) -> Option<CellStyleRef> {
        self.get_style_by_name_(&Self::make_currency_style_name_(currency_name))
    }

    fn get_date_style(&self, date_format: &str) -> Option<CellStyleRef> {
        self.get_style_by_name_(&Self::make_date_style_name_(date_format))
    }
}

impl Output for OdsOutput<'_> {
    fn write(&mut self) -> Result<(), Error> {
        debug!("create style");
        self.create_style()?;

        debug!("write summary");
        self.write_summary();

        if self.details_sheet {
            debug!("write trades");
            self.write_trades();

            debug!("write close positions");
            self.write_close_positions();

            debug!("write heat map");
            self.write_heat_map();

            debug!("write distribution");
            self.write_distribution();

            debug!("write position indicators");
            self.write_position_indicators();

            for instrument_name in self.portfolio.get_instrument_name_list() {
                for position_index in self.indicators.get_position_index_list(instrument_name) {
                    debug!(
                        "write position indicators for {} / {}",
                        instrument_name, position_index
                    );
                    let position_indicators = self
                        .indicators
                        .get_position_indicators(instrument_name, position_index);
                    self.write_position_instrument_indicators(position_indicators);
                }
            }
        }

        debug!("save");
        self.save()?;
        Ok(())
    }
}
