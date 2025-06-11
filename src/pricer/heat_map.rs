use chrono::Datelike;

use super::{PortfolioIndicator, PortfolioIndicators, PositionIndicator, PositionIndicators};
use crate::alias::Date;

pub enum HeatMapPeriod {
    Monthly,
    Yearly,
}

impl HeatMapPeriod {
    fn same(&self, left: Date, right: Date) -> bool {
        match self {
            HeatMapPeriod::Monthly => left.month() == right.month() && left.year() == right.year(),
            HeatMapPeriod::Yearly => left.year() == right.year(),
        }
    }
}

pub enum HeatMapComputeMode {
    Percent,
    Delta,
    Value,
}

impl HeatMapComputeMode {
    fn compute(&self, left: f64, right: f64) -> f64 {
        match self {
            HeatMapComputeMode::Value => left - right,
            HeatMapComputeMode::Delta => (left + 1.0) / (right + 1.0) - 1.0,
            HeatMapComputeMode::Percent => (left - right) / left,
        }
    }
}
pub struct HeatMap {
    pub data: Vec<(Date, f64)>,
    pub period: HeatMapPeriod,
    pub mode: HeatMapComputeMode,
}

impl HeatMap {
    pub fn from_portfolios<T>(
        indicators: &PortfolioIndicators,
        period: HeatMapPeriod,
        mode: HeatMapComputeMode,
        get_value: T,
    ) -> Self
    where
        T: Fn(&PortfolioIndicator) -> f64,
    {
        Self::from_(
            &indicators.portfolios,
            period,
            mode,
            get_value,
            |indicator| indicator.date,
        )
    }

    pub fn from_positions<T>(
        indicators: &PositionIndicators,
        period: HeatMapPeriod,
        mode: HeatMapComputeMode,
        get_value: T,
    ) -> Self
    where
        T: Fn(&PositionIndicator) -> f64,
    {
        Self::from_(
            &indicators.positions,
            period,
            mode,
            |indicator| get_value(indicator),
            |indicator| indicator.date,
        )
    }

    fn from_<I, D, V>(
        indicators: &[I],
        period: HeatMapPeriod,
        mode: HeatMapComputeMode,
        get_value: V,
        get_date: D,
    ) -> Self
    where
        V: Fn(&I) -> f64,
        D: Fn(&I) -> Date,
    {
        let mut data = Vec::new();
        let mut ref_value = 0.0;
        let mut ref_date = None;

        for values in indicators.windows(2) {
            let current_indicator = &values[0];
            let next_indicator = &values[1];
            let current_date = get_date(current_indicator);
            let next_date = get_date(next_indicator);
            if !period.same(next_date, current_date) {
                data.push((
                    current_date,
                    mode.compute(get_value(current_indicator), ref_value),
                ));
                ref_value = get_value(current_indicator);
                ref_date = Some(current_date);
            }
        }

        if let Some(last_indicator) = indicators.last() {
            let last_date = get_date(last_indicator);
            let last_value = get_value(last_indicator);
            if let Some(date) = ref_date {
                if date != last_date {
                    data.push((last_date, mode.compute(last_value, ref_value)));
                }
            } else {
                data.push((last_date, last_value));
            }
        }

        HeatMap { data, period, mode }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_float_eq::*;

    fn make_date_(year: i32, month: u32, day: u32) -> Date {
        Date::from_ymd_opt(year, month, day).unwrap()
    }

    fn check_monthly_<B>(builder: B)
    where
        B: Fn(HeatMapPeriod, HeatMapComputeMode) -> HeatMap,
    {
        let heat_map_delta = builder(HeatMapPeriod::Monthly, HeatMapComputeMode::Delta);
        assert!(
            heat_map_delta.data.len() == 3,
            "heat_map_delta.data.len() = {}",
            heat_map_delta.data.len()
        );

        let heat_map_value = builder(HeatMapPeriod::Monthly, HeatMapComputeMode::Value);
        assert!(
            heat_map_value.data.len() == 3,
            "heat_map_value.data.len() = {}",
            heat_map_value.data.len()
        );

        let heat_map_percent = builder(HeatMapPeriod::Monthly, HeatMapComputeMode::Percent);
        assert!(
            heat_map_percent.data.len() == 3,
            "heat_map_percent.data.len() = {}",
            heat_map_percent.data.len()
        );

        for (i, (wanted_date, wanted_value_delta, wanted_value_value, wanted_value_percent)) in [
            (make_date_(2023, 9, 26), 0.6, 0.6, 1.0),
            (
                make_date_(2023, 10, 31),
                (0.2 + 1.0) / (0.6 + 1.0) - 1.0,
                0.2 - 0.6,
                (0.2 - 0.6) / 0.2,
            ),
            (
                make_date_(2023, 11, 2),
                (0.8 + 1.0) / (0.2 + 1.0) - 1.0,
                0.8 - 0.2,
                (0.8 - 0.2) / 0.8,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            dbg!(
                i,
                wanted_date,
                wanted_value_delta,
                wanted_value_value,
                heat_map_delta.data[i].0,
                heat_map_delta.data[i].1,
                heat_map_value.data[i].0,
                heat_map_value.data[i].1,
                heat_map_percent.data[i].0,
                heat_map_percent.data[i].1,
            );

            assert!(heat_map_delta.data[i].0 == wanted_date);
            assert_float_absolute_eq!(heat_map_delta.data[i].1, wanted_value_delta, 1e-7);

            assert!(heat_map_value.data[i].0 == wanted_date);
            assert_float_absolute_eq!(heat_map_value.data[i].1, wanted_value_value, 1e-7);

            assert!(heat_map_percent.data[i].0 == wanted_date);
            assert_float_absolute_eq!(heat_map_percent.data[i].1, wanted_value_percent, 1e-7);
        }
    }

    fn check_yearly_<B>(builder: B)
    where
        B: Fn(HeatMapPeriod, HeatMapComputeMode) -> HeatMap,
    {
        let heat_map_delta = builder(HeatMapPeriod::Yearly, HeatMapComputeMode::Delta);
        assert!(
            heat_map_delta.data.len() == 3,
            "heat_map_delta.data.len() = {}",
            heat_map_delta.data.len()
        );

        let heat_map_value = builder(HeatMapPeriod::Yearly, HeatMapComputeMode::Value);
        assert!(
            heat_map_value.data.len() == 3,
            "heat_map_value.data.len() = {}",
            heat_map_value.data.len()
        );

        let heat_map_percent = builder(HeatMapPeriod::Yearly, HeatMapComputeMode::Percent);
        assert!(
            heat_map_percent.data.len() == 3,
            "heat_map_percent.data.len() = {}",
            heat_map_percent.data.len()
        );

        for (i, (wanted_date, wanted_value_delta, wanted_value_value, wanted_value_percent)) in [
            (make_date_(2022, 9, 25), 0.5, 0.5, 1.0),
            (
                make_date_(2023, 12, 20),
                (0.9 + 1.0) / (0.5 + 1.0) - 1.0,
                0.9 - 0.5,
                (0.9 - 0.5) / 0.9,
            ),
            (
                make_date_(2024, 1, 3),
                (0.4 + 1.0) / (0.9 + 1.0) - 1.0,
                0.4 - 0.9,
                (0.4 - 0.9) / 0.4,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            dbg!(
                i,
                wanted_date,
                wanted_value_delta,
                wanted_value_value,
                heat_map_delta.data[i].0,
                heat_map_delta.data[i].1,
                heat_map_value.data[i].0,
                heat_map_value.data[i].1,
                heat_map_value.data[i].1,
                heat_map_percent.data[i].0,
                heat_map_percent.data[i].1,
            );

            assert!(heat_map_delta.data[i].0 == wanted_date);
            assert_float_absolute_eq!(heat_map_delta.data[i].1, wanted_value_delta, 1e-7);

            assert!(heat_map_value.data[i].0 == wanted_date);
            assert_float_absolute_eq!(heat_map_value.data[i].1, wanted_value_value, 1e-7);

            assert!(heat_map_percent.data[i].0 == wanted_date);
            assert_float_absolute_eq!(heat_map_percent.data[i].1, wanted_value_percent, 1e-7);
        }
    }

    #[test]
    fn monthly() {
        let input = [
            (make_date_(2023, 9, 25), 0.5),
            (make_date_(2023, 9, 26), 0.6),
            (make_date_(2023, 10, 30), 0.0),
            (make_date_(2023, 10, 31), 0.2),
            (make_date_(2023, 11, 1), 0.7),
            (make_date_(2023, 11, 2), 0.8),
        ];

        {
            let positions = input
                .iter()
                .map(|(date, value)| PositionIndicator {
                    date: *date,
                    earning: *value,
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            let positions = positions.iter().collect();
            let indicators = PositionIndicators {
                positions,
                ..Default::default()
            };

            check_monthly_(|period, mode| {
                HeatMap::from_positions(&indicators, period, mode, |item| item.earning)
            });
        }

        {
            let portfolios = input
                .iter()
                .map(|(date, value)| PortfolioIndicator {
                    date: *date,
                    earning: *value,
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            let indicators = PortfolioIndicators {
                portfolios,
                ..Default::default()
            };

            check_monthly_(|period, mode| {
                HeatMap::from_portfolios(&indicators, period, mode, |item| item.earning)
            });
        }
    }

    #[test]
    fn yearly() {
        let input = [
            (make_date_(2022, 9, 25), 0.5),
            (make_date_(2023, 1, 6), 0.7),
            (make_date_(2023, 12, 20), 0.9),
            (make_date_(2024, 1, 3), 0.4),
        ];

        {
            let positions = input
                .iter()
                .map(|(date, value)| PositionIndicator {
                    date: *date,
                    earning: *value,
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            let positions = positions.iter().collect();
            let indicators = PositionIndicators {
                positions,
                ..Default::default()
            };

            check_yearly_(|period, mode| {
                HeatMap::from_positions(&indicators, period, mode, |item| item.earning)
            });
        }

        {
            let portfolios = input
                .iter()
                .map(|(date, value)| PortfolioIndicator {
                    date: *date,
                    earning: *value,
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            let indicators = PortfolioIndicators {
                portfolios,
                ..Default::default()
            };

            check_yearly_(|period, mode| {
                HeatMap::from_portfolios(&indicators, period, mode, |item| item.earning)
            });
        }
    }

    #[test]
    fn empty() {
        let input: Vec<(Date, f64)> = Default::default();
        let heat_map = HeatMap::from_(
            &input,
            HeatMapPeriod::Yearly,
            HeatMapComputeMode::Delta,
            |indicator| indicator.1,
            |indicator| indicator.0,
        );
        assert!(
            heat_map.data.is_empty(),
            "heat_map.data.len() = {}",
            heat_map.data.len()
        );
    }

    #[test]
    fn one() {
        let input = vec![(make_date_(2023, 9, 25), 0.5)];
        let heat_map = HeatMap::from_(
            &input,
            HeatMapPeriod::Monthly,
            HeatMapComputeMode::Delta,
            |indicator| indicator.1,
            |indicator| indicator.0,
        );
        assert!(
            heat_map.data.len() == 1,
            "heat_map.data.len() = {}",
            heat_map.data.len()
        );
        for (i, (wanted_date, wanted_value)) in
            [(make_date_(2023, 9, 25), 0.5)].into_iter().enumerate()
        {
            dbg!(
                i,
                wanted_date,
                wanted_value,
                heat_map.data[i].0,
                heat_map.data[i].1
            );
            assert!(heat_map.data[i].0 == wanted_date);
            assert_float_absolute_eq!(heat_map.data[i].1, wanted_value, 1e-7);
        }
    }
}
