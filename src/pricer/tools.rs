use crate::alias::Date;
use chrono::Datelike;

#[derive(Copy, Clone)]
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

    pub fn instant(begin_valuation: f64, end_valuation: f64) -> Self {
        Self {
            value: end_valuation - begin_valuation,
            value_pct: (end_valuation - begin_valuation) / begin_valuation,
        }
    }

    pub fn accumulate(current: Pnl, previous: Pnl) -> Self {
        Self {
            value: current.value + previous.value,
            value_pct: (current.value_pct + 1.0) * (previous.value_pct + 1.0) - 1.0,
        }
    }

    pub fn instant_with_cashflow(cashflow: f64, begin_valuation: f64, end_valuation: f64) -> Self {
        Self::instant(begin_valuation + cashflow, end_valuation)
    }
}

impl Default for Pnl {
    fn default() -> Self {
        Pnl::zero()
    }
}

fn same_week(date1: Date, date2: Date) -> bool {
    let min_date = std::cmp::min(&date1, &date2);
    let max_date = std::cmp::max(&date1, &date2);
    (*max_date - *min_date).num_days() < 7
        && max_date.weekday().num_days_from_monday() > min_date.weekday().num_days_from_monday()
}

fn same_month(date1: Date, date2: Date) -> bool {
    date1.month() == date2.month()
}

fn same_year(date1: Date, date2: Date) -> bool {
    date1.year() == date2.year()
}

fn is_3_month(date1: Date, date2: Date) -> bool {
    (date1 - date2).num_days().abs() < 30 * 3
}

fn is_1_year(date1: Date, date2: Date) -> bool {
    (date1 - date2).num_days().abs() < 365
}

fn make_volatility(values: Vec<f64>) -> f64 {
    let size = values.len() as f64;
    let avg = values.iter().sum::<f64>() / size;
    values
        .iter()
        .map(|value| (value - avg) * (value - avg) / size)
        .sum::<f64>()
        .sqrt()
}

pub struct PnlAccumulator {
    pub date: Date,
    pub valuation: f64,
    pub daily: Pnl,
    pub weekly: Pnl,
    pub monthly: Pnl,
    pub yearly: Pnl,
    pub global: Pnl,
    pub for_3_months: Pnl,
    pub for_1_year: Pnl,
    pub volatility_3_month: f64,
    pub volatility_1_year: f64,
    pub previous_daily: Vec<(Date, Pnl)>,
}

impl PnlAccumulator {
    pub fn zero() -> Self {
        let pnl = Pnl::zero();
        Self {
            date: Date::default(),
            valuation: 0.0,
            daily: pnl,
            weekly: pnl,
            monthly: pnl,
            yearly: pnl,
            global: pnl,
            for_3_months: pnl,
            for_1_year: pnl,
            volatility_3_month: 0.0,
            volatility_1_year: 0.0,
            previous_daily: Default::default(),
        }
    }

    pub fn append(&mut self, date: Date, cashflow: f64, valuation: f64) {
        assert!(self.date < date);
        if valuation == 0.0 {
            *self = PnlAccumulator::zero();
            return;
        };
        self.daily = Pnl::instant_with_cashflow(cashflow, self.valuation, valuation);
        self.global = Pnl::accumulate(self.daily, self.global);
        self.weekly = if same_week(self.date, date) {
            Pnl::accumulate(self.daily, self.weekly)
        } else {
            self.daily
        };
        self.monthly = if same_month(self.date, date) && same_year(self.date, date) {
            Pnl::accumulate(self.daily, self.monthly)
        } else {
            self.daily
        };
        self.yearly = if same_year(self.date, date) {
            Pnl::accumulate(self.daily, self.yearly)
        } else {
            self.daily
        };
        self.valuation = valuation;
        self.date = date;
        self.previous_daily.push((date, self.daily));
        self.previous_daily.retain(|item| is_1_year(date, item.0));
        self.for_1_year = self
            .previous_daily
            .iter()
            .fold(Pnl::zero(), |pnl, item| Pnl::accumulate(pnl, item.1));
        self.for_3_months = self
            .previous_daily
            .iter()
            .filter(|item| is_3_month(date, item.0))
            .fold(Pnl::zero(), |pnl, item| Pnl::accumulate(pnl, item.1));
        self.volatility_1_year = make_volatility(
            self.previous_daily
                .iter()
                .map(|(_, pnl)| pnl.value_pct)
                .collect(),
        );
        self.volatility_3_month = make_volatility(
            self.previous_daily
                .iter()
                .filter(|item| is_3_month(date, item.0))
                .map(|(_, pnl)| pnl.value_pct)
                .collect(),
        );
    }
}

impl Default for PnlAccumulator {
    fn default() -> Self {
        PnlAccumulator::zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_float_eq::*;

    #[test]
    fn same_week_month_year() {
        assert!(same_year(make_date_(2023, 11, 10), make_date_(2023, 1, 10)));
        assert!(!same_year(
            make_date_(2023, 11, 10),
            make_date_(2022, 1, 10)
        ));

        assert!(same_month(
            make_date_(2023, 11, 10),
            make_date_(2023, 11, 1)
        ));
        assert!(same_month(
            make_date_(2023, 11, 10),
            make_date_(2022, 11, 1)
        ));
        assert!(!same_month(
            make_date_(2023, 11, 10),
            make_date_(2023, 1, 1)
        ));

        assert!(same_week(make_date_(2023, 11, 6), make_date_(2023, 11, 7)));
        assert!(same_week(make_date_(2023, 11, 6), make_date_(2023, 11, 12)));
        assert!(!same_week(
            make_date_(2023, 11, 6),
            make_date_(2023, 11, 13)
        ));
        assert!(!same_week(
            make_date_(2023, 11, 6),
            make_date_(2023, 11, 20)
        ));
    }

    #[test]
    fn is_1_year_3_month() {
        assert!(is_1_year(make_date_(2023, 11, 10), make_date_(2023, 1, 10)));
        assert!(is_1_year(
            make_date_(2023, 11, 10),
            make_date_(2022, 11, 20)
        ));
        assert!(!is_1_year(
            make_date_(2023, 11, 10),
            make_date_(2022, 11, 9)
        ));

        assert!(is_3_month(
            make_date_(2023, 11, 10),
            make_date_(2023, 11, 20)
        ));
        assert!(is_3_month(make_date_(2023, 11, 10), make_date_(2023, 9, 2)));
        assert!(!is_3_month(
            make_date_(2023, 11, 10),
            make_date_(2023, 8, 2)
        ));
    }
    #[test]
    fn pnl_accumulator_01() {
        let mut acc = PnlAccumulator::zero();
        acc.append(make_date_(2023, 11, 7), 1050.0, 905.0);
        check_pnl(acc.daily, -145.0, -145.0 / 1050.0);
        check_pnl(acc.global, -145.0, -145.0 / 1050.0);
        check_pnl(acc.yearly, -145.0, -145.0 / 1050.0);
        check_pnl(acc.monthly, -145.0, -145.0 / 1050.0);
        check_pnl(acc.weekly, -145.0, -145.0 / 1050.0);
        check_pnl(acc.for_1_year, -145.0, -145.0 / 1050.0);
        check_pnl(acc.for_3_months, -145.0, -145.0 / 1050.0);
    }

    #[test]
    fn pnl_accumulator_02() {
        let mut acc = PnlAccumulator::zero();
        acc.append(make_date_(2023, 11, 6), 1000.0, 1020.0);
        check_pnl(acc.daily, 20.0, 0.02);
        check_pnl(acc.global, 20.0, 0.02);
        check_pnl(acc.yearly, 20.0, 0.02);
        check_pnl(acc.monthly, 20.0, 0.02);
        check_pnl(acc.weekly, 20.0, 0.02);
        check_pnl(acc.for_1_year, 20.0, 0.02);
        check_pnl(acc.for_3_months, 20.0, 0.02);

        acc.append(make_date_(2023, 11, 7), 100.0, 1142.40);
        check_pnl(acc.daily, 22.4, 0.02);
        check_pnl(acc.global, 42.4, 0.0404);
        check_pnl(acc.yearly, 42.4, 0.0404);
        check_pnl(acc.monthly, 42.4, 0.0404);
        check_pnl(acc.weekly, 42.4, 0.0404);
        check_pnl(acc.for_1_year, 42.4, 0.0404);
        check_pnl(acc.for_3_months, 42.4, 0.0404);

        acc.append(make_date_(2023, 11, 8), 200.0, 1369.25);
        check_pnl(acc.daily, 26.85, 0.02);
        check_pnl(acc.global, 69.25, 0.0612);
        check_pnl(acc.yearly, 69.25, 0.0612);
        check_pnl(acc.monthly, 69.25, 0.0612);
        check_pnl(acc.weekly, 69.25, 0.0612);
        check_pnl(acc.for_1_year, 69.25, 0.0612);
        check_pnl(acc.for_3_months, 69.25, 0.0612);

        acc.append(make_date_(2023, 11, 9), 0.0, 1396.63);
        check_pnl(acc.daily, 27.38, 0.02);
        check_pnl(acc.global, 96.63, 0.0824);
        check_pnl(acc.yearly, 96.63, 0.0824);
        check_pnl(acc.monthly, 96.63, 0.0824);
        check_pnl(acc.weekly, 96.63, 0.0824);
        check_pnl(acc.for_1_year, 96.63, 0.0824);
        check_pnl(acc.for_3_months, 96.63, 0.0824);
    }

    #[test]
    fn pnl_accumulator_03() {
        let mut acc = PnlAccumulator::zero();
        acc.append(make_date_(2023, 11, 6), 1000.0, 1020.0);
        check_pnl(acc.daily, 20.0, 0.02);
        check_pnl(acc.global, 20.0, 0.02);
        check_pnl(acc.yearly, 20.0, 0.02);
        check_pnl(acc.monthly, 20.0, 0.02);
        check_pnl(acc.weekly, 20.0, 0.02);
        check_pnl(acc.for_1_year, 20.0, 0.02);
        check_pnl(acc.for_3_months, 20.0, 0.02);

        acc.append(make_date_(2023, 11, 7), 200.0, 1244.40);
        check_pnl(acc.daily, 24.4, 0.02);
        check_pnl(acc.global, 44.4, 0.0404);
        check_pnl(acc.yearly, 44.4, 0.0404);
        check_pnl(acc.monthly, 44.4, 0.0404);
        check_pnl(acc.weekly, 44.4, 0.0404);
        check_pnl(acc.for_1_year, 44.4, 0.0404);
        check_pnl(acc.for_3_months, 44.4, 0.0404);

        acc.append(make_date_(2023, 11, 8), -100.0, 1167.29);
        check_pnl(acc.daily, 22.89, 0.02);
        check_pnl(acc.global, 67.29, 0.0612);
        check_pnl(acc.yearly, 67.29, 0.0612);
        check_pnl(acc.monthly, 67.29, 0.0612);
        check_pnl(acc.weekly, 67.29, 0.0612);
        check_pnl(acc.for_1_year, 67.29, 0.0612);
        check_pnl(acc.for_3_months, 67.29, 0.0612);

        acc.append(make_date_(2023, 11, 9), 0.0, 1190.63);
        check_pnl(acc.daily, 23.34, 0.02);
        check_pnl(acc.global, 90.63, 0.0824);
        check_pnl(acc.yearly, 90.63, 0.0824);
        check_pnl(acc.monthly, 90.63, 0.0824);
        check_pnl(acc.weekly, 90.63, 0.0824);
        check_pnl(acc.for_1_year, 90.63, 0.0824);
        check_pnl(acc.for_3_months, 90.63, 0.0824);
    }

    #[test]
    fn pnl_accumulator_04() {
        let mut acc = PnlAccumulator::zero();
        acc.append(make_date_(2023, 11, 2), 1000.0, 1000.0 * 1.02);
        check_pnl(acc.daily, 20.0, 0.02);
        check_pnl(acc.global, 20.0, 0.02);
        check_pnl(acc.yearly, 20.0, 0.02);
        check_pnl(acc.monthly, 20.0, 0.02);
        check_pnl(acc.weekly, 20.0, 0.02);
        check_pnl(acc.for_1_year, 20.0, 0.02);
        check_pnl(acc.for_3_months, 20.0, 0.02);

        acc.append(make_date_(2023, 11, 3), 0.0, 1000.0 * 1.02 * 1.02);
        check_pnl(acc.daily, 20.4, 0.02);
        check_pnl(acc.global, 40.4, 0.0404);
        check_pnl(acc.yearly, 40.4, 0.0404);
        check_pnl(acc.monthly, 40.4, 0.0404);
        check_pnl(acc.weekly, 40.4, 0.0404);
        check_pnl(acc.for_1_year, 40.4, 0.0404);
        check_pnl(acc.for_3_months, 40.4, 0.0404);

        acc.append(make_date_(2023, 11, 6), 0.0, 1000.0 * 1.02 * 1.02 * 1.02);
        check_pnl(acc.daily, 20.808, 0.02);
        check_pnl(acc.global, 61.208, 0.0612);
        check_pnl(acc.yearly, 61.208, 0.0612);
        check_pnl(acc.monthly, 61.208, 0.0612);
        check_pnl(acc.weekly, 20.808, 0.02);
        check_pnl(acc.for_1_year, 61.208, 0.0612);
        check_pnl(acc.for_3_months, 61.208, 0.0612);

        acc.append(
            make_date_(2023, 11, 7),
            0.0,
            1000.0 * 1.02 * 1.02 * 1.02 * 1.02,
        );
        check_pnl(acc.daily, 21.22416, 0.02);
        check_pnl(acc.global, 82.4322, 0.0824);
        check_pnl(acc.yearly, 82.4322, 0.0824);
        check_pnl(acc.monthly, 82.4322, 0.0824);
        check_pnl(acc.weekly, 42.03216, 0.0404);
        check_pnl(acc.for_1_year, 82.4322, 0.0824);
        check_pnl(acc.for_3_months, 82.4322, 0.0824);
    }

    #[test]
    fn pnl_accumulator_05() {
        let mut acc = PnlAccumulator::zero();
        acc.append(make_date_(2023, 10, 30), 1000.0, 1000.0 * 1.02);
        check_pnl(acc.daily, 20.0, 0.02);
        check_pnl(acc.global, 20.0, 0.02);
        check_pnl(acc.yearly, 20.0, 0.02);
        check_pnl(acc.monthly, 20.0, 0.02);
        check_pnl(acc.weekly, 20.0, 0.02);
        check_pnl(acc.for_1_year, 20.0, 0.02);
        check_pnl(acc.for_3_months, 20.0, 0.02);

        acc.append(make_date_(2023, 10, 31), 0.0, 1000.0 * 1.02 * 1.02);
        check_pnl(acc.daily, 20.4, 0.02);
        check_pnl(acc.global, 40.4, 0.0404);
        check_pnl(acc.yearly, 40.4, 0.0404);
        check_pnl(acc.monthly, 40.4, 0.0404);
        check_pnl(acc.weekly, 40.4, 0.0404);
        check_pnl(acc.for_1_year, 40.4, 0.0404);
        check_pnl(acc.for_3_months, 40.4, 0.0404);

        acc.append(make_date_(2023, 11, 1), 0.0, 1000.0 * 1.02 * 1.02 * 1.02);
        check_pnl(acc.daily, 20.808, 0.02);
        check_pnl(acc.global, 61.208, 0.0612);
        check_pnl(acc.yearly, 61.208, 0.0612);
        check_pnl(acc.monthly, 20.808, 0.02);
        check_pnl(acc.weekly, 61.208, 0.0612);
        check_pnl(acc.for_1_year, 61.208, 0.0612);
        check_pnl(acc.for_3_months, 61.208, 0.0612);

        acc.append(
            make_date_(2023, 11, 2),
            0.0,
            1000.0 * 1.02 * 1.02 * 1.02 * 1.02,
        );
        check_pnl(acc.daily, 21.22416, 0.02);
        check_pnl(acc.global, 82.4322, 0.0824);
        check_pnl(acc.yearly, 82.4322, 0.0824);
        check_pnl(acc.monthly, 42.03216, 0.0404);
        check_pnl(acc.weekly, 82.4322, 0.0824);
        check_pnl(acc.for_1_year, 82.4322, 0.0824);
        check_pnl(acc.for_3_months, 82.4322, 0.0824);
    }

    #[test]
    fn pnl_accumulator_06() {
        let mut acc = PnlAccumulator::zero();
        acc.append(make_date_(2021, 12, 30), 1000.0, 1000.0 * 1.02);
        check_pnl(acc.daily, 20.0, 0.02);
        check_pnl(acc.global, 20.0, 0.02);
        check_pnl(acc.yearly, 20.0, 0.02);
        check_pnl(acc.monthly, 20.0, 0.02);
        check_pnl(acc.weekly, 20.0, 0.02);
        check_pnl(acc.for_1_year, 20.0, 0.02);
        check_pnl(acc.for_3_months, 20.0, 0.02);

        acc.append(make_date_(2021, 12, 31), 0.0, 1000.0 * 1.02 * 1.02);
        check_pnl(acc.daily, 20.4, 0.02);
        check_pnl(acc.global, 40.4, 0.0404);
        check_pnl(acc.yearly, 40.4, 0.0404);
        check_pnl(acc.monthly, 40.4, 0.0404);
        check_pnl(acc.weekly, 40.4, 0.0404);
        check_pnl(acc.for_1_year, 40.4, 0.0404);
        check_pnl(acc.for_3_months, 40.4, 0.0404);

        acc.append(make_date_(2022, 1, 1), 0.0, 1000.0 * 1.02 * 1.02 * 1.02);
        check_pnl(acc.daily, 20.808, 0.02);
        check_pnl(acc.global, 61.208, 0.0612);
        check_pnl(acc.yearly, 20.808, 0.02);
        check_pnl(acc.monthly, 20.808, 0.02);
        check_pnl(acc.weekly, 61.208, 0.0612);
        check_pnl(acc.for_1_year, 61.208, 0.0612);
        check_pnl(acc.for_3_months, 61.208, 0.0612);

        acc.append(
            make_date_(2022, 1, 2),
            0.0,
            1000.0 * 1.02 * 1.02 * 1.02 * 1.02,
        );
        check_pnl(acc.daily, 21.22416, 0.02);
        check_pnl(acc.global, 82.4322, 0.0824);
        check_pnl(acc.yearly, 42.03216, 0.0404);
        check_pnl(acc.monthly, 42.03216, 0.0404);
        check_pnl(acc.weekly, 82.4322, 0.0824);
        check_pnl(acc.for_1_year, 82.4322, 0.0824);
        check_pnl(acc.for_3_months, 82.4322, 0.0824);
    }

    #[test]
    fn pnl_accumulator_07() {
        let mut acc = PnlAccumulator::zero();
        acc.append(make_date_(2021, 1, 1), 1000.0, 1000.0 * 1.02);
        check_pnl(acc.daily, 20.0, 0.02);
        check_pnl(acc.global, 20.0, 0.02);
        check_pnl(acc.for_3_months, 20.0, 0.02);

        acc.append(make_date_(2021, 2, 1), 0.0, 1000.0 * 1.02 * 1.02);
        check_pnl(acc.daily, 20.4, 0.02);
        check_pnl(acc.global, 40.4, 0.0404);
        check_pnl(acc.for_3_months, 40.4, 0.0404);

        acc.append(make_date_(2021, 3, 1), 0.0, 1000.0 * 1.02 * 1.02 * 1.02);
        check_pnl(acc.daily, 20.808, 0.02);
        check_pnl(acc.global, 61.208, 0.0612);
        check_pnl(acc.for_3_months, 61.208, 0.0612);

        acc.append(
            make_date_(2021, 4, 1),
            0.0,
            1000.0 * 1.02 * 1.02 * 1.02 * 1.02,
        );
        check_pnl(acc.daily, 21.22416, 0.02);
        check_pnl(acc.global, 82.4322, 0.0824);
        check_pnl(acc.for_3_months, 62.4321, 0.0612);
    }

    #[test]
    fn pnl_accumulator_08() {
        let mut acc = PnlAccumulator::zero();
        acc.append(make_date_(2021, 1, 1), 1000.0, 1000.0 * 1.02);
        check_pnl(acc.daily, 20.0, 0.02);
        check_pnl(acc.global, 20.0, 0.02);
        check_pnl(acc.for_1_year, 20.0, 0.02);

        acc.append(make_date_(2021, 6, 1), 0.0, 1000.0 * 1.02 * 1.02);
        check_pnl(acc.daily, 20.4, 0.02);
        check_pnl(acc.global, 40.4, 0.0404);
        check_pnl(acc.for_1_year, 40.4, 0.0404);

        acc.append(make_date_(2021, 12, 1), 0.0, 1000.0 * 1.02 * 1.02 * 1.02);
        check_pnl(acc.daily, 20.808, 0.02);
        check_pnl(acc.global, 61.208, 0.0612);
        check_pnl(acc.for_1_year, 61.208, 0.0612);

        acc.append(
            make_date_(2022, 4, 1),
            0.0,
            1000.0 * 1.02 * 1.02 * 1.02 * 1.02,
        );
        check_pnl(acc.daily, 21.22416, 0.02);
        check_pnl(acc.global, 82.4322, 0.0824);
        check_pnl(acc.for_1_year, 62.4321, 0.0612);
    }

    #[test]
    fn make_volatility_01() {
        let res = make_volatility(Vec::default());
        assert_float_absolute_eq!(res, f64::default(), 1e-7);
    }

    #[test]
    fn make_volatility_02() {
        let res = make_volatility(vec![1.0, 2.5, 10.0, 7.0]);
        assert_float_absolute_eq!(res, 3.5772720053135463, 1e-7);
    }

    fn make_date_(year: i32, month: u32, day: u32) -> Date {
        Date::from_ymd_opt(year, month, day).unwrap()
    }

    fn check_pnl(pnl: Pnl, value: f64, pct: f64) {
        assert_float_absolute_eq!(pnl.value, value, 1e-4);
        assert_float_absolute_eq!(pnl.value_pct, pct, 1e-4);
    }
}
