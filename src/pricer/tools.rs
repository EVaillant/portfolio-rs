use crate::alias::Date;
use chrono::Datelike;

enum Delay {
    Zero,
    Days(chrono::naive::Days),
    Months(chrono::Months),
}

impl Delay {
    pub fn zero() -> Self {
        Self::Zero
    }
    pub fn days(num: u64) -> Self {
        Self::Days(chrono::naive::Days::new(num))
    }

    pub fn months(num: u32) -> Self {
        Self::Months(chrono::Months::new(num))
    }

    pub fn sub(&self, date: &Date) -> Option<Date> {
        match self {
            Self::Zero => Some(*date),
            Self::Days(delay) => date.checked_sub_days(*delay),
            Self::Months(delay) => date.checked_sub_months(*delay),
        }
    }
}

pub struct Pnl {
    pub value: f64,
    pub value_pct: f64,
}

impl Pnl {
    pub fn zero() -> Self {
        Self {
            value: 0.0,
            value_pct: 0.0,
        }
    }

    pub fn new(nominal: f64, valuation: f64) -> Self {
        Self {
            value: valuation - nominal,
            value_pct: (valuation - nominal) / nominal,
        }
    }

    pub fn relative(
        previous_nominal: f64,
        previous_valuation: f64,
        current_nominal: f64,
        current_valuation: f64,
    ) -> Self {
        Self {
            value: (current_valuation - previous_valuation) - (current_nominal - previous_nominal),
            value_pct: ((current_valuation - previous_valuation)
                - (current_nominal - previous_nominal))
                / (current_valuation - (current_nominal - previous_nominal)),
        }
    }
}

impl Default for Pnl {
    fn default() -> Self {
        Pnl::zero()
    }
}

pub fn make_pnls<T>(
    date: Date,
    nominal: f64,
    valuation: f64,
    get_previous_value: T,
) -> (Pnl, Pnl, Pnl, Pnl, Pnl, Pnl, Pnl)
where
    T: Fn(Date) -> Option<(f64, f64)>,
{
    let pnl_current = if valuation == 0.0 {
        Default::default()
    } else {
        Pnl::new(nominal, valuation)
    };

    let pnl_yearly = Date::from_ymd_opt(date.year() - 1, 12, 31).map(|previous_year_date| {
        make_pnl(
            previous_year_date,
            Delay::zero(),
            nominal,
            valuation,
            &get_previous_value,
        )
    });

    (
        pnl_current,
        make_pnl(
            date,
            Delay::days(1),
            nominal,
            valuation,
            &get_previous_value,
        ),
        make_pnl(
            date,
            Delay::days((date.weekday().num_days_from_monday() + 1) as u64),
            nominal,
            valuation,
            &get_previous_value,
        ),
        make_pnl(
            date,
            Delay::days(date.day() as u64),
            nominal,
            valuation,
            &get_previous_value,
        ),
        pnl_yearly.unwrap_or_default(),
        make_pnl(
            date,
            Delay::months(3),
            nominal,
            valuation,
            &get_previous_value,
        ),
        make_pnl(
            date,
            Delay::months(12),
            nominal,
            valuation,
            &get_previous_value,
        ),
    )
}

fn make_pnl<T>(
    date: Date,
    delay: Delay,
    current_nominal: f64,
    current_valuation: f64,
    get_previous_value: &T,
) -> Pnl
where
    T: Fn(Date) -> Option<(f64, f64)>,
{
    if current_valuation == 0.0 {
        Default::default()
    } else if let Some(previous_date) = delay.sub(&date) {
        if let Some((previous_nominal, previous_valuation)) = get_previous_value(previous_date) {
            Pnl::relative(
                previous_nominal,
                previous_valuation,
                current_nominal,
                current_valuation,
            )
        } else {
            Pnl::new(current_nominal, current_valuation)
        }
    } else {
        Default::default()
    }
}

pub fn make_volatilities<T>(date: Date, get_previous_value: T) -> (f64, f64)
where
    T: Fn(Date) -> Vec<f64>,
{
    (
        make_volatility(date, Delay::months(3), &get_previous_value),
        make_volatility(date, Delay::months(12), &get_previous_value),
    )
}

fn make_volatility<T>(date: Date, delay: Delay, get_previous_value: &T) -> f64
where
    T: Fn(Date) -> Vec<f64>,
{
    if let Some(previous_date) = delay.sub(&date) {
        let values = get_previous_value(previous_date);
        let size = values.len() as f64;
        let avg = values.iter().sum::<f64>() / size;
        values
            .iter()
            .map(|value| (value - avg) * (value - avg) / size)
            .sum::<f64>()
            .sqrt()
    } else {
        Default::default()
    }
}
