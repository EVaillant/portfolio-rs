#[derive(Debug, PartialEq, Eq)]
pub enum Way {
    Buy,
    Sell,
}

impl std::fmt::Display for Way {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Way::Buy => write!(f, "Buy"),
            Way::Sell => write!(f, "Sell"),
        }
    }
}
