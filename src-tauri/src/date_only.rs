use chrono::{Datelike, NaiveDate};

/// Parse a strict `YYYY-MM-DD` calendar date without any timezone conversion.
///
/// Returns `None` for malformed values, out-of-range years, and impossible
/// calendar dates such as `2026-02-30`.
pub fn parse_date_only(value: &str) -> Option<NaiveDate> {
    let bytes = value.as_bytes();

    if bytes.len() != 10 {
        return None;
    }

    for (index, byte) in bytes.iter().enumerate() {
        if index == 4 || index == 7 {
            if *byte != b'-' {
                return None;
            }
        } else if !byte.is_ascii_digit() {
            return None;
        }
    }

    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()?;

    (1..=9999).contains(&date.year()).then_some(date)
}

pub fn format_date_only(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// A first run must not invent a preparation history. Use the current local
/// day, clamped to the exam date so the initial record always satisfies the
/// schema's ordering constraint.
pub fn default_preparation_start(today: NaiveDate, exam_date: NaiveDate) -> NaiveDate {
    today.min(exam_date)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    #[test]
    fn accepts_real_calendar_dates() {
        assert_eq!(parse_date_only("2026-12-19"), Some(date(2026, 12, 19)));
        assert_eq!(parse_date_only("2024-02-29"), Some(date(2024, 2, 29)));
        assert_eq!(parse_date_only("2000-02-29"), Some(date(2000, 2, 29)));
    }

    #[test]
    fn rejects_invalid_dates_and_formats() {
        for value in [
            "2026-02-30",
            "2026-02-29",
            "1900-02-29",
            "2026-04-31",
            "2026-13-01",
            "2026-00-10",
            "2026-12-00",
            "2026-2-03",
            "26-12-19",
            "2026/12/19",
            " 2026-12-19",
            "2026-12-19 ",
            "",
        ] {
            assert_eq!(parse_date_only(value), None, "should reject {value:?}");
        }
    }

    #[test]
    fn formats_dates_without_timezone_conversion() {
        assert_eq!(format_date_only(date(2026, 12, 19)), "2026-12-19");
        assert_eq!(format_date_only(date(2024, 2, 29)), "2024-02-29");
    }

    #[test]
    fn first_run_start_never_predates_today_or_exceeds_exam() {
        let exam = date(2026, 12, 19);

        assert_eq!(
            default_preparation_start(date(2026, 10, 29), exam),
            date(2026, 10, 29)
        );
        assert_eq!(
            default_preparation_start(date(2027, 1, 2), exam),
            exam,
            "the default is clamped when the first run is after the exam date"
        );
    }
}
