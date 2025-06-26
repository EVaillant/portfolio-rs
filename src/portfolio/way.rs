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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display() {
        let buy = Way::Buy;
        let sell = Way::Sell;
        assert!(buy.to_string() == "Buy");
        assert!(sell.to_string() == "Sell");
    }
}
