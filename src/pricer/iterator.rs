use crate::alias::Date;
use crate::pricer::Step;
use chrono::Datelike;

pub struct DateByStepIterator {
    it: Option<Date>,
    end: Date,
    step: Step,
}

impl DateByStepIterator {
    pub fn new(begin: Date, end: Date, step: Step) -> Self {
        let it = match step {
            Step::Day => Some(begin),
            Step::Week => {
                let days_from_monday = begin.weekday().num_days_from_monday() as u64;
                begin.checked_add_days(chrono::naive::Days::new(6 - days_from_monday))
            }
            Step::Month => Date::from_ymd_opt(begin.year(), begin.month(), 1)
                .and_then(|v| v.checked_add_months(chrono::Months::new(1)))
                .and_then(|v| v.checked_sub_days(chrono::naive::Days::new(1))),
            Step::Year => Date::from_ymd_opt(begin.year(), 12, 31),
        };
        Self { it, end, step }
    }
}

impl Iterator for DateByStepIterator {
    type Item = Date;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(v) = self.it {
            if v >= self.end {
                self.it = None;

                Some(self.end)
            } else {
                match self.step {
                    Step::Day => {
                        self.it = v.checked_add_days(chrono::naive::Days::new(1));
                    }
                    Step::Week => {
                        self.it = v.checked_add_days(chrono::naive::Days::new(7));
                    }
                    Step::Year => {
                        self.it = Date::from_ymd_opt(v.year() + 1, 12, 31);
                    }
                    Step::Month => {
                        self.it = v
                            .checked_add_days(chrono::naive::Days::new(1))
                            .and_then(|v| v.checked_add_months(chrono::Months::new(1)))
                            .and_then(|v| v.checked_sub_days(chrono::naive::Days::new(1)));
                    }
                }
                Some(v)
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_by_step_01() {
        let it = DateByStepIterator::new(
            Date::from_ymd_opt(2020, 5, 4).unwrap(),
            Date::from_ymd_opt(2020, 5, 10).unwrap(),
            Step::Day,
        );
        let result = it.collect::<Vec<_>>();
        let wanted = vec![
            Date::from_ymd_opt(2020, 5, 4).unwrap(),
            Date::from_ymd_opt(2020, 5, 5).unwrap(),
            Date::from_ymd_opt(2020, 5, 6).unwrap(),
            Date::from_ymd_opt(2020, 5, 7).unwrap(),
            Date::from_ymd_opt(2020, 5, 8).unwrap(),
            Date::from_ymd_opt(2020, 5, 9).unwrap(),
            Date::from_ymd_opt(2020, 5, 10).unwrap(),
        ];
        assert_eq!(result, wanted);
    }

    #[test]
    fn date_by_step_02() {
        let it = DateByStepIterator::new(
            Date::from_ymd_opt(2023, 4, 5).unwrap(),
            Date::from_ymd_opt(2023, 5, 3).unwrap(),
            Step::Week,
        );
        let result = it.collect::<Vec<_>>();
        let wanted = vec![
            Date::from_ymd_opt(2023, 4, 9).unwrap(),
            Date::from_ymd_opt(2023, 4, 16).unwrap(),
            Date::from_ymd_opt(2023, 4, 23).unwrap(),
            Date::from_ymd_opt(2023, 4, 30).unwrap(),
            Date::from_ymd_opt(2023, 5, 3).unwrap(),
        ];
        assert_eq!(result, wanted);
    }

    #[test]
    fn date_by_step_03() {
        let it = DateByStepIterator::new(
            Date::from_ymd_opt(2021, 4, 5).unwrap(),
            Date::from_ymd_opt(2023, 5, 3).unwrap(),
            Step::Year,
        );
        let result = it.collect::<Vec<_>>();
        let wanted = vec![
            Date::from_ymd_opt(2021, 12, 31).unwrap(),
            Date::from_ymd_opt(2022, 12, 31).unwrap(),
            Date::from_ymd_opt(2023, 5, 3).unwrap(),
        ];
        assert_eq!(result, wanted);
    }

    #[test]
    fn date_by_step_04() {
        let it = DateByStepIterator::new(
            Date::from_ymd_opt(2021, 1, 5).unwrap(),
            Date::from_ymd_opt(2021, 6, 10).unwrap(),
            Step::Month,
        );
        let result = it.collect::<Vec<_>>();
        let wanted = vec![
            Date::from_ymd_opt(2021, 1, 31).unwrap(),
            Date::from_ymd_opt(2021, 2, 28).unwrap(),
            Date::from_ymd_opt(2021, 3, 31).unwrap(),
            Date::from_ymd_opt(2021, 4, 30).unwrap(),
            Date::from_ymd_opt(2021, 5, 31).unwrap(),
            Date::from_ymd_opt(2021, 6, 10).unwrap(),
        ];
        assert_eq!(result, wanted);
    }
}
