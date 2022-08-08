use crate::marketdata::Instrument;
use crate::portfolio::Trade;
use std::rc::Rc;

#[derive(Debug)]
pub struct Position {
    pub instrument: Rc<Instrument>,
    pub trades: Vec<Trade>,
}
