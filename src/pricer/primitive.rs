use crate::alias::{Date, Duration};

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

pub fn volatility(values: &[f64]) -> f64 {
    if !values.is_empty() {
        let size = values.len() as f64;
        let avg = values.iter().sum::<f64>() / size;
        values
            .iter()
            .map(|value| (value - avg) * (value - avg) / size)
            .sum::<f64>()
            .sqrt()
    } else {
        0.0
    }
}

pub fn volatility_from<D, G, F>(
    date: Date,
    delay: Duration,
    datas: &[D],
    current_value: f64,
    get_value: G,
    get_date: F,
) -> f64
where
    G: Fn(&D) -> f64,
    F: Fn(&D) -> Date,
{
    let mut values = datas
        .iter()
        .filter(|data| get_date(data) + delay > date)
        .map(get_value)
        .collect::<Vec<_>>();
    values.push(current_value);
    volatility(&values)
}

pub fn xirr(cash_flows: &[(Date, f64)], guess: f64) -> Option<f64> {
    let max_iterations = 100;
    let precision = 1e-7;
    let mut rate = guess;
    let d0 = cash_flows.first()?.0;

    for _ in 0..max_iterations {
        let mut f = 0.0;
        let mut df = 0.0;

        for cf in cash_flows {
            let days = (cf.0 - d0).num_days() as f64;
            let frac = days / 365.0;
            let denom = (1.0 + rate).powf(frac);
            f += cf.1 / denom;
            df += -cf.1 * frac / denom / (1.0 + rate);
        }

        let new_rate = rate - f / df;
        if (new_rate - rate).abs() < precision {
            return Some(new_rate);
        }

        rate = new_rate;
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::alias::{Date, Duration};
    use assert_float_eq::*;

    fn make_date_(year: i32, month: u32, day: u32) -> Date {
        Date::from_ymd_opt(year, month, day).unwrap()
    }

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

    #[test]
    fn volatility() {
        {
            let result = super::volatility(&[]);
            assert_float_absolute_eq!(result, 0.0, 1e-7);
        }

        {
            let result = super::volatility(&[1.0, 5.0, 9.0, 8.0, 6.0]);
            assert_float_absolute_eq!(result, 2.785677655436824, 1e-7);
        }
    }

    #[test]
    fn volatility_from() {
        let result = super::volatility_from(
            make_date_(2025, 6, 10),
            Duration::days(5),
            &[
                (make_date_(2025, 6, 9), 1.0),
                (make_date_(2025, 6, 8), 5.0),
                (make_date_(2025, 6, 6), 8.0),
                (make_date_(2025, 6, 2), 6.0),
                (make_date_(2025, 6, 1), 7.0),
            ],
            1.0,
            |item| item.1,
            |item| item.0,
        );
        assert_float_absolute_eq!(result, 2.9474565306379, 1e-7);
    }

    #[test]
    fn xirr_01() {
        let flows = [
            (make_date_(2023, 1, 1), -1000.0),
            (make_date_(2023, 12, 31), 500.0),
            (make_date_(2024, 12, 31), 600.0),
        ];

        let result = super::xirr(&flows, 0.1).unwrap();
        assert_float_absolute_eq!(result, 0.06399657333732633, 1e-7);
    }

    #[test]
    fn xirr_02() {
        let flows = [
            (make_date_(2023, 1, 1), 0.0),
            (make_date_(2023, 12, 31), 0.0),
        ];

        assert!(super::xirr(&flows, 0.1).is_none());
    }

    #[test]
    fn xirr_03() {
        let flows = [];
        assert!(super::xirr(&flows, 0.1).is_none());
    }

    #[test]
    fn xirr_04() {
        let flows = [(make_date_(2023, 1, 1), -1000.0)];
        assert!(super::xirr(&flows, 0.1).is_none());
    }
}
