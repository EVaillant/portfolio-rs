pub fn pnl(valuation: f64, nominal: f64) -> (f64, f64) {
    let pnl_currency = valuation - nominal;
    let pnl_percent = if nominal.abs() < 1e-7 {
        0.0
    } else {
        pnl_currency / nominal
    };
    (pnl_currency, pnl_percent)
}

pub fn twr(begin_valuation: f64, end_valuation: f64, cashflow: f64, previous_twr: f64) -> f64 {
    let period_twr = if begin_valuation.abs() < 1e-7 {
        0.0
    } else {
        (end_valuation - begin_valuation - cashflow) / begin_valuation
    };

    (previous_twr + 1.0) * (period_twr + 1.0) - 1.0
}

#[cfg(test)]
mod tests {
    use assert_float_eq::*;

    #[test]
    fn pnl() {
        {
            let (currency, percent) = super::pnl(1500.0, 1000.0);
            assert_float_absolute_eq!(currency, 500.0, 1e-7);
            assert_float_absolute_eq!(percent, 0.5, 1e-7);
        }
        {
            let (currency, percent) = super::pnl(1500.0, 0.0);
            assert_float_absolute_eq!(currency, 1500.0, 1e-7);
            assert_float_absolute_eq!(percent, 0.0, 1e-7);
        }
    }

    #[test]
    fn twr() {
        assert_float_absolute_eq!(super::twr(0.0, 1000.0, 950.0, 0.0), 0.0, 1e-7);
        assert_float_absolute_eq!(super::twr(1000.0, 1500.0, 200.0, 0.0), 0.3, 1e-7);
        assert_float_absolute_eq!(super::twr(1000.0, 1500.0, 200.0, 0.5), 0.95, 1e-7);
        assert_float_absolute_eq!(super::twr(1000.0, 200.0, -1000.0, 0.0), 0.20, 1e-7);
    }
}
