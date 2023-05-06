use crate::alias::Date;
use crate::error::Error;
use crate::historical::Provider;
use crate::portfolio::Portfolio;

use log::{debug, info};

mod pnl;
mod portfolio;
mod position;

pub use portfolio::PortfolioIndicator;
pub use position::PositionIndicator;

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
                    spot_provider.fetch(&position.instrument, instrument_begin, end)?;
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
