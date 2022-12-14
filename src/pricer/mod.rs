use crate::alias::DateTime;
use crate::marketdata::Instrument;
use crate::portfolio::{Portfolio, Position, Way};

use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct PositionIndicator {
    pub date: DateTime,
    pub unit_price: f64,
    pub quantity: u32,
    pub tax: f64,
    pub dividends: f64,
}

impl PositionIndicator {
    pub fn from_position(position: &Position, date: DateTime) -> Option<Self> {
        let mut unit_price = 0.0;
        let mut quantity = 0;
        let mut tax = 0.0;

        for trade in position.trades.iter() {
            if trade.date > date {
                break;
            }
            match trade.way {
                Way::Sell => {
                    quantity -= trade.quantity;
                }
                Way::Buy => {
                    unit_price = (quantity as f64 * unit_price
                        + trade.price * trade.quantity as f64
                        + trade.tax)
                        / (quantity as f64 + trade.quantity as f64);
                    quantity += trade.quantity;
                }
            };
            tax += trade.tax;
        }

        let dividends = position
            .instrument
            .dividends
            .as_ref()
            .map_or(0.0, |values| {
                values
                    .iter()
                    .map(|item| {
                        if item.date > date {
                            0.0
                        } else {
                            let mut quantity = 0;
                            for trade in position.trades.iter() {
                                if trade.date > item.date {
                                    break;
                                }
                                match trade.way {
                                    Way::Sell => {
                                        quantity -= trade.quantity;
                                    }
                                    Way::Buy => {
                                        quantity += trade.quantity;
                                    }
                                };
                            }
                            item.value * quantity as f64
                        }
                    })
                    .sum()
            });

        if quantity == 0 {
            None
        } else {
            Some(Self {
                date,
                unit_price,
                quantity,
                tax,
                dividends,
            })
        }
    }

    pub fn pnl(&self, price: f64) -> f64 {
        self.quantity as f64 * (price - self.unit_price)
    }

    pub fn valuations(&self) -> f64 {
        self.quantity as f64 * self.unit_price
    }
}

#[derive(Debug)]
pub struct PortfolioIndicator {
    pub date: DateTime,
    pub positions: HashMap<Rc<Instrument>, PositionIndicator>,
}

impl PortfolioIndicator {
    pub fn from_portfolio(portfolio: &Portfolio, date: DateTime) -> Self {
        let mut positions = HashMap::new();
        for position in portfolio.positions.iter() {
            if let Some(indicator) = PositionIndicator::from_position(position, date) {
                positions.insert(position.instrument.clone(), indicator);
            }
        }
        Self { date, positions }
    }

    pub fn valuations(&self) -> f64 {
        self.positions
            .values()
            .map(|position| position.valuations())
            .sum()
    }

    pub fn pnl<F>(&self, spotter: F) -> Option<f64>
    where
        F: Fn(&Instrument, DateTime) -> Option<f64>,
    {
        self.positions
            .iter()
            .map(|(instrument, position_indicator)| {
                spotter(instrument, self.date).map(|price| position_indicator.pnl(price))
            })
            .fold(Some(0.0), |accu, value| match (accu, value) {
                (None, _) => None,
                (_, None) => None,
                (Some(l), Some(r)) => Some(l + r),
            })
    }
}
