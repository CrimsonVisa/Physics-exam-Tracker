use chrono::{Days, NaiveDate};

pub const ALGORITHM_VERSION: i64 = 1;
pub const INITIAL_EASE: f64 = 2.5;
pub const MIN_EASE: f64 = 1.3;
pub const MAX_EASE: f64 = 3.0;
pub const MAX_INTERVAL_DAYS: i64 = 3650;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rating {
    Again,
    Hard,
    Good,
    Easy,
}

impl Rating {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Again => "again",
            Self::Hard => "hard",
            Self::Good => "good",
            Self::Easy => "easy",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "again" => Some(Self::Again),
            "hard" => Some(Self::Hard),
            "good" => Some(Self::Good),
            "easy" => Some(Self::Easy),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScheduleState {
    pub interval_days: i64,
    pub ease_factor: f64,
    pub review_count: i64,
    pub lapse_count: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScheduleOutcome {
    pub interval_days: i64,
    pub ease_factor: f64,
    pub review_count: i64,
    pub lapse_count: i64,
    pub due_on: NaiveDate,
}

/// Deterministic, date-only scheduling inspired by SM-2. The rating changes
/// the interval and ease factor; there is no randomness and no wall-clock
/// dependency beyond the supplied local review date.
pub fn schedule_review(state: ScheduleState, rating: Rating, today: NaiveDate) -> ScheduleOutcome {
    let is_first_review = state.review_count == 0 || state.interval_days <= 0;
    let mut ease = state.ease_factor.clamp(MIN_EASE, MAX_EASE);
    let mut lapses = state.lapse_count;

    let interval = if is_first_review {
        match rating {
            Rating::Again | Rating::Hard => 1,
            Rating::Good => 2,
            Rating::Easy => 4,
        }
    } else {
        let current = state.interval_days.max(1);
        match rating {
            Rating::Again => 1,
            Rating::Hard => (current + 1).max(round_days(current as f64 * 1.20)),
            Rating::Good => 3.max(round_days(current as f64 * ease)),
            Rating::Easy => 4.max(round_days(current as f64 * ease * 1.30)),
        }
    };

    match rating {
        Rating::Again => {
            ease -= 0.20;
            lapses += 1;
        }
        Rating::Hard => ease -= 0.15,
        Rating::Good => {}
        Rating::Easy => ease += 0.15,
    }

    let interval_days = interval.clamp(1, MAX_INTERVAL_DAYS);
    let ease_factor = round_ease(ease);
    let due_on = today
        .checked_add_days(Days::new(interval_days as u64))
        .unwrap_or(today);

    ScheduleOutcome {
        interval_days,
        ease_factor,
        review_count: state.review_count + 1,
        lapse_count: lapses.min(state.review_count + 1),
        due_on,
    }
}

fn round_days(value: f64) -> i64 {
    value.round() as i64
}

fn round_ease(value: f64) -> f64 {
    ((value * 100.0).round() / 100.0).clamp(MIN_EASE, MAX_EASE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").expect("date")
    }

    fn state(
        interval_days: i64,
        ease_factor: f64,
        review_count: i64,
        lapse_count: i64,
    ) -> ScheduleState {
        ScheduleState {
            interval_days,
            ease_factor,
            review_count,
            lapse_count,
        }
    }

    #[test]
    fn first_review_intervals_and_ease_are_deterministic() {
        let today = date("2026-11-08");
        let outcomes = [
            (Rating::Again, 1, 2.30, 1),
            (Rating::Hard, 1, 2.35, 0),
            (Rating::Good, 2, 2.50, 0),
            (Rating::Easy, 4, 2.65, 0),
        ];
        for (rating, interval, ease, lapses) in outcomes {
            let outcome = schedule_review(state(0, INITIAL_EASE, 0, 0), rating, today);
            assert_eq!(outcome.interval_days, interval);
            assert_eq!(outcome.ease_factor, ease);
            assert_eq!(outcome.lapse_count, lapses);
            assert_eq!(outcome.review_count, 1);
            assert_eq!(
                outcome.due_on,
                date("2026-11-09") + chrono::Duration::days(interval - 1)
            );
        }
    }

    #[test]
    fn subsequent_intervals_follow_the_documented_formulas() {
        let today = date("2026-11-08");
        let existing = state(5, 2.5, 3, 0);

        let hard = schedule_review(existing, Rating::Hard, today);
        assert_eq!(hard.interval_days, 6);
        assert_eq!(hard.ease_factor, 2.35);
        assert_eq!(hard.due_on, date("2026-11-14"));

        let good = schedule_review(existing, Rating::Good, today);
        assert_eq!(good.interval_days, 13);
        assert_eq!(good.ease_factor, 2.5);
        assert_eq!(good.due_on, date("2026-11-21"));

        let easy = schedule_review(existing, Rating::Easy, today);
        assert_eq!(easy.interval_days, 16);
        assert_eq!(easy.ease_factor, 2.65);
        assert_eq!(easy.due_on, date("2026-11-24"));

        let again = schedule_review(existing, Rating::Again, today);
        assert_eq!(again.interval_days, 1);
        assert_eq!(again.ease_factor, 2.3);
        assert_eq!(again.lapse_count, 1);
        assert_eq!(again.due_on, date("2026-11-09"));
    }

    #[test]
    fn good_progression_matches_the_worked_example() {
        let today = date("2026-11-08");
        let first = schedule_review(state(0, INITIAL_EASE, 0, 0), Rating::Good, today);
        assert_eq!(first.interval_days, 2);
        let second = schedule_review(
            state(
                first.interval_days,
                first.ease_factor,
                first.review_count,
                first.lapse_count,
            ),
            Rating::Good,
            date("2026-11-10"),
        );
        assert_eq!(second.interval_days, 5);
        let third = schedule_review(
            state(
                second.interval_days,
                second.ease_factor,
                second.review_count,
                second.lapse_count,
            ),
            Rating::Good,
            date("2026-11-15"),
        );
        assert_eq!(third.interval_days, 13);
        let fourth = schedule_review(
            state(
                third.interval_days,
                third.ease_factor,
                third.review_count,
                third.lapse_count,
            ),
            Rating::Easy,
            date("2026-11-28"),
        );
        assert_eq!(fourth.interval_days, 42);
        assert_eq!(fourth.ease_factor, 2.65);
    }

    #[test]
    fn ease_and_interval_are_clamped() {
        let today = date("2026-11-08");
        let low = schedule_review(state(10, MIN_EASE, 4, 2), Rating::Again, today);
        assert_eq!(low.ease_factor, MIN_EASE);
        assert_eq!(low.interval_days, 1);

        let high = schedule_review(
            state(MAX_INTERVAL_DAYS, MAX_EASE, 10, 0),
            Rating::Easy,
            today,
        );
        assert_eq!(high.interval_days, MAX_INTERVAL_DAYS);
        assert_eq!(high.ease_factor, MAX_EASE);
        assert_eq!(high.due_on, date("2036-11-05"));
    }

    #[test]
    fn dates_cross_month_and_leap_year_boundaries() {
        let outcome = schedule_review(state(2, 2.5, 1, 0), Rating::Good, date("2028-02-27"));
        assert_eq!(outcome.due_on, date("2028-03-03"));
    }
}
