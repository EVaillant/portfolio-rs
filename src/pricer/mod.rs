use crate::alias::Date;
use crate::error::Error;
use crate::historical::Provider;
use crate::portfolio::Portfolio;
use chrono::Datelike;
use std::collections::BTreeMap;

use log::{debug, info};

mod portfolio;
mod position;
mod tools;

pub use portfolio::PortfolioIndicator;
pub use position::PositionIndicator;

fn is_last_day_of_month(date: Date) -> bool {
    Date::from_ymd_opt(date.year(), date.month(), 1)
        .and_then(|v| v.checked_add_months(chrono::Months::new(1)))
        .and_then(|v| v.checked_sub_days(chrono::naive::Days::new(1)))
        .map_or(false, |v| v == date)
}

pub struct HeatMapItem {
    data: [Option<f64>; 12],
}

impl HeatMapItem {
    pub fn new() -> Self {
        Self {
            data: Default::default(),
        }
    }

    #[inline]
    pub fn data(&self) -> &[Option<f64>; 12] {
        &self.data
    }

    #[inline]
    pub fn update(&mut self, month: usize, value: f64) {
        self.data[month] = Some(value);
    }
}

pub struct PortfolioIndicators {
    pub portfolios: Vec<PortfolioIndicator>,
}

impl PortfolioIndicators {
    pub fn from_portfolio<P>(
        portfolio: &Portfolio,
        begin: Date,
        end: Date,
        spot_provider: &mut P,
    ) -> Result<PortfolioIndicators, Error>
    where
        P: Provider,
    {
        info!(
            "request all market data historical for {} from {} to {} pricing",
            portfolio.name,
            begin.format("%Y-%m-%d"),
            end.format("%Y-%m-%d"),
        );

        for position in portfolio.positions.iter() {
            if let Some(trade) = position.trades.first() {
                let instrument_begin = trade.date.date();
                if instrument_begin < end {
                    let instrument_end = position
                        .get_close_date()
                        .map(|date_time| date_time.date())
                        .unwrap_or(end);
                    spot_provider.fetch(&position.instrument, instrument_begin, instrument_end)?;
                }
            }
        }
        info!("request all market data historical done");

        info!("start to price portfolios");
        let portfolios =
            PortfolioIndicators::make_portfolios_(portfolio, begin, end, spot_provider);
        info!("price portfolios is finished");

        Ok(PortfolioIndicators { portfolios })
    }

    pub fn by_instrument_name(&self, instrument_name: &str) -> Vec<&PositionIndicator> {
        self.portfolios
            .iter()
            .flat_map(|item| {
                item.positions
                    .iter()
                    .find(|item_position| item_position.instrument.name == instrument_name)
            })
            .collect()
    }

    pub fn make_heat_map(&self) -> BTreeMap<i32, HeatMapItem> {
        let mut values = self
            .portfolios
            .iter()
            .filter(|item| is_last_day_of_month(item.date))
            .collect::<Vec<_>>();
        if let Some(last) = self.portfolios.last() {
            if !is_last_day_of_month(last.date) {
                values.push(last);
            }
        }
        let mut lines: BTreeMap<i32, HeatMapItem> = Default::default();
        for item in values {
            let year = item.date.year();
            lines
                .entry(year)
                .or_insert_with(HeatMapItem::new)
                .update(item.date.month0() as usize, item.pnl_monthly.value_pct)
        }
        lines
    }

    pub fn make_instrument_heat_map(&self, instrument_name: &str) -> BTreeMap<i32, HeatMapItem> {
        let position_by_instrument = self.by_instrument_name(instrument_name);
        let mut values = position_by_instrument
            .iter()
            .filter(|item| is_last_day_of_month(item.date))
            .collect::<Vec<_>>();
        if let Some(last) = position_by_instrument.last() {
            if !is_last_day_of_month(last.date) {
                values.push(last);
            }
        }
        let mut lines: BTreeMap<i32, HeatMapItem> = Default::default();
        for item in values {
            let year = item.date.year();
            lines
                .entry(year)
                .or_insert_with(HeatMapItem::new)
                .update(item.date.month0() as usize, item.pnl_monthly.value_pct)
        }
        lines
    }

    fn make_portfolios_<P>(
        portfolio: &Portfolio,
        begin: Date,
        end: Date,
        spot_provider: &mut P,
    ) -> Vec<PortfolioIndicator>
    where
        P: Provider,
    {
        let mut data = Vec::new();
        let mut it = begin;
        while it <= end {
            let value = PortfolioIndicator::from_portfolio(portfolio, it, spot_provider, &data);
            if !value.positions.is_empty() {
                data.push(value);
            } else {
                debug!("pricing result at {} is ignored (position empty)", it);
            }
            if let Some(next_it) = it.checked_add_days(chrono::naive::Days::new(1)) {
                it = next_it;
            } else {
                break;
            }
        }
        data
    }
}
