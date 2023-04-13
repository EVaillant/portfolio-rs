use crate::alias::Date;
use chrono::naive::Days;
use chrono::Datelike;

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
) -> (Pnl, Pnl, Pnl, Pnl, Pnl)
where
    T: Fn(Date, Days) -> Option<(f64, f64)>,
{
    let current_pnl = if valuation == 0.0 {
        Default::default()
    } else {
        Pnl::new(nominal, valuation)
    };

    let yearly_pnl = Date::from_ymd_opt(date.year() - 1, 12, 31).map(|previous_year_date| {
        make_pnl(
            previous_year_date,
            Days::new(0),
            nominal,
            valuation,
            &get_previous_value,
        )
    });

    (
        current_pnl,
        make_pnl(date, Days::new(1), nominal, valuation, &get_previous_value),
        make_pnl(
            date,
            Days::new((date.weekday().num_days_from_monday() + 1) as u64),
            nominal,
            valuation,
            &get_previous_value,
        ),
        make_pnl(
            date,
            Days::new(date.day() as u64),
            nominal,
            valuation,
            &get_previous_value,
        ),
        yearly_pnl.unwrap_or_default(),
    )
}

fn make_pnl<T>(
    date: Date,
    delta: Days,
    current_nominal: f64,
    current_valuation: f64,
    get_previous_value: &T,
) -> Pnl
where
    T: Fn(Date, Days) -> Option<(f64, f64)>,
{
    if current_valuation == 0.0 {
        Default::default()
    } else if let Some((previous_nominal, previous_valuation)) = get_previous_value(date, delta) {
        Pnl::relative(
            previous_nominal,
            previous_valuation,
            current_nominal,
            current_valuation,
        )
    } else {
        Pnl::new(current_nominal, current_valuation)
    }
}
