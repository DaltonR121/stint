//! Relative and absolute date parsing.

use time::format_description::FormatItem;
use time::macros::format_description;
use time::{Duration, OffsetDateTime, Weekday};

/// ISO date format: YYYY-MM-DD.
const ISO_DATE_FMT: &[FormatItem<'static>] = format_description!("[year]-[month]-[day]");

/// Parses a date string into an `OffsetDateTime` at midnight UTC.
///
/// Supported formats:
/// - ISO date: "2026-01-15"
/// - Relative: "today", "yesterday"
/// - Day reference: "last monday", "last tuesday", etc.
///
/// The `now` parameter is injected for testability.
pub fn parse_date(input: &str, now: OffsetDateTime) -> Result<OffsetDateTime, String> {
    let input = input.trim().to_lowercase();

    match input.as_str() {
        "today" => Ok(midnight(now)),
        "yesterday" => Ok(midnight(now) - Duration::days(1)),
        _ => {
            // Try "last <weekday>"
            if let Some(day_str) = input.strip_prefix("last ") {
                let target = parse_weekday(day_str.trim())?;
                return Ok(last_weekday(now, target));
            }

            // Try ISO date
            let date = time::Date::parse(&input, ISO_DATE_FMT)
                .map_err(|_| format!("unrecognized date format: '{input}'"))?;
            Ok(date.midnight().assume_utc())
        }
    }
}

/// Returns the given datetime truncated to midnight UTC.
fn midnight(dt: OffsetDateTime) -> OffsetDateTime {
    dt.date().midnight().assume_utc()
}

/// Parses a weekday name string.
fn parse_weekday(s: &str) -> Result<Weekday, String> {
    match s {
        "monday" | "mon" => Ok(Weekday::Monday),
        "tuesday" | "tue" | "tues" => Ok(Weekday::Tuesday),
        "wednesday" | "wed" => Ok(Weekday::Wednesday),
        "thursday" | "thu" | "thurs" => Ok(Weekday::Thursday),
        "friday" | "fri" => Ok(Weekday::Friday),
        "saturday" | "sat" => Ok(Weekday::Saturday),
        "sunday" | "sun" => Ok(Weekday::Sunday),
        _ => Err(format!("unknown day: '{s}'")),
    }
}

/// Returns midnight of the most recent occurrence of the given weekday before `now`.
///
/// If today is the target weekday, returns 7 days ago (always goes back).
fn last_weekday(now: OffsetDateTime, target: Weekday) -> OffsetDateTime {
    let today = now.weekday();
    let days_back = match (today.number_days_from_monday() as i64)
        - (target.number_days_from_monday() as i64)
    {
        diff if diff > 0 => diff,
        diff if diff <= 0 => diff + 7,
        _ => unreachable!(),
    };
    midnight(now) - Duration::days(days_back)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    // Wednesday, 2026-03-11 14:30:00 UTC
    const NOW: OffsetDateTime = datetime!(2026-03-11 14:30:00 UTC);

    #[test]
    fn parse_today() {
        let result = parse_date("today", NOW).unwrap();
        assert_eq!(result, datetime!(2026-03-11 0:00 UTC));
    }

    #[test]
    fn parse_yesterday() {
        let result = parse_date("yesterday", NOW).unwrap();
        assert_eq!(result, datetime!(2026-03-10 0:00 UTC));
    }

    #[test]
    fn parse_iso_date() {
        let result = parse_date("2026-01-15", NOW).unwrap();
        assert_eq!(result, datetime!(2026-01-15 0:00 UTC));
    }

    #[test]
    fn parse_last_monday() {
        // NOW is Wednesday, so last Monday is 2 days ago
        let result = parse_date("last monday", NOW).unwrap();
        assert_eq!(result, datetime!(2026-03-09 0:00 UTC));
    }

    #[test]
    fn parse_last_friday() {
        // NOW is Wednesday, so last Friday is 5 days ago
        let result = parse_date("last friday", NOW).unwrap();
        assert_eq!(result, datetime!(2026-03-06 0:00 UTC));
    }

    #[test]
    fn parse_last_wednesday_goes_back_7() {
        // NOW is Wednesday, so "last wednesday" = 7 days ago
        let result = parse_date("last wednesday", NOW).unwrap();
        assert_eq!(result, datetime!(2026-03-04 0:00 UTC));
    }

    #[test]
    fn parse_abbreviated_day() {
        let result = parse_date("last mon", NOW).unwrap();
        assert_eq!(result, datetime!(2026-03-09 0:00 UTC));
    }

    #[test]
    fn parse_case_insensitive() {
        let result = parse_date("Yesterday", NOW).unwrap();
        assert_eq!(result, datetime!(2026-03-10 0:00 UTC));
    }

    #[test]
    fn parse_with_whitespace() {
        let result = parse_date("  today  ", NOW).unwrap();
        assert_eq!(result, datetime!(2026-03-11 0:00 UTC));
    }

    #[test]
    fn parse_invalid_errors() {
        assert!(parse_date("not-a-date", NOW).is_err());
    }

    #[test]
    fn parse_unknown_day_errors() {
        assert!(parse_date("last blorpday", NOW).is_err());
    }
}
