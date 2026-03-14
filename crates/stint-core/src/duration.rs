//! Human-friendly duration parsing.

/// Parses a human-friendly duration string into total seconds.
///
/// Supported formats: "2h30m", "45m", "1h", "90s", "1h30m15s", "2h 30m".
/// Units: `h` (hours), `m` (minutes), `s` (seconds).
/// At least one unit must be present.
pub fn parse_duration(input: &str) -> Result<i64, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("duration cannot be empty".to_string());
    }

    let mut total_secs: i64 = 0;
    let mut current_num = String::new();
    let mut found_unit = false;

    for ch in input.chars() {
        if ch.is_ascii_digit() {
            current_num.push(ch);
        } else if ch == ' ' {
            // Allow spaces between components
            continue;
        } else {
            let unit = ch.to_ascii_lowercase();
            if current_num.is_empty() {
                return Err(format!("expected a number before '{unit}' in '{input}'"));
            }
            let value: i64 = current_num
                .parse()
                .map_err(|_| format!("invalid number in '{input}'"))?;

            match unit {
                'h' => total_secs += value * 3600,
                'm' => total_secs += value * 60,
                's' => total_secs += value,
                _ => {
                    return Err(format!(
                        "unknown unit '{unit}' in '{input}' (use h, m, or s)"
                    ))
                }
            }

            current_num.clear();
            found_unit = true;
        }
    }

    // Trailing number without a unit
    if !current_num.is_empty() {
        return Err(format!(
            "missing unit after '{current_num}' in '{input}' (use h, m, or s)"
        ));
    }

    if !found_unit {
        return Err(format!("no valid duration found in '{input}'"));
    }

    if total_secs == 0 {
        return Err("duration must be greater than zero".to_string());
    }

    Ok(total_secs)
}

/// Formats a duration in seconds as a human-readable string (e.g., "2h 30m").
///
/// Shows hours and minutes. Seconds are only shown if the duration is under a minute
/// or if there is a non-zero seconds component.
pub fn format_duration_human(secs: i64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;

    if h > 0 && m > 0 && s > 0 {
        format!("{h}h {m}m {s}s")
    } else if h > 0 && m > 0 {
        format!("{h}h {m}m")
    } else if h > 0 && s > 0 {
        format!("{h}h {s}s")
    } else if h > 0 {
        format!("{h}h")
    } else if m > 0 && s > 0 {
        format!("{m}m {s}s")
    } else if m > 0 {
        format!("{m}m")
    } else {
        format!("{s}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hours_only() {
        assert_eq!(parse_duration("2h").unwrap(), 7200);
    }

    #[test]
    fn parse_minutes_only() {
        assert_eq!(parse_duration("45m").unwrap(), 2700);
    }

    #[test]
    fn parse_seconds_only() {
        assert_eq!(parse_duration("90s").unwrap(), 90);
    }

    #[test]
    fn parse_hours_and_minutes() {
        assert_eq!(parse_duration("2h30m").unwrap(), 9000);
    }

    #[test]
    fn parse_hours_minutes_seconds() {
        assert_eq!(parse_duration("1h30m15s").unwrap(), 5415);
    }

    #[test]
    fn parse_with_spaces() {
        assert_eq!(parse_duration("2h 30m").unwrap(), 9000);
    }

    #[test]
    fn parse_uppercase() {
        assert_eq!(parse_duration("2H30M").unwrap(), 9000);
    }

    #[test]
    fn parse_empty_errors() {
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn parse_no_unit_errors() {
        assert!(parse_duration("30").is_err());
    }

    #[test]
    fn parse_zero_errors() {
        assert!(parse_duration("0h").is_err());
    }

    #[test]
    fn parse_unknown_unit_errors() {
        assert!(parse_duration("5d").is_err());
    }

    #[test]
    fn parse_unit_without_number_errors() {
        assert!(parse_duration("h").is_err());
    }

    #[test]
    fn format_hours_and_minutes() {
        assert_eq!(format_duration_human(5400), "1h 30m");
    }

    #[test]
    fn format_minutes_only() {
        assert_eq!(format_duration_human(300), "5m");
    }

    #[test]
    fn format_seconds_only() {
        assert_eq!(format_duration_human(45), "45s");
    }

    #[test]
    fn format_hours_only() {
        assert_eq!(format_duration_human(7200), "2h");
    }

    #[test]
    fn format_zero() {
        assert_eq!(format_duration_human(0), "0s");
    }

    #[test]
    fn format_all_components() {
        assert_eq!(format_duration_human(3661), "1h 1m 1s");
    }
}
