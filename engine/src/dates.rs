//! A card's date is milliseconds since 1970; the date field of the card dialog speaks `YYYY-MM-DD`.

use chrono::{Local, NaiveDate, TimeZone, Utc};

/// The local calendar day of a timestamp, `YYYY-MM-DD` (empty for a timestamp that is not a date).
pub fn ymd(ms: i64) -> String {
    Local.timestamp_millis_opt(ms).single().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()
}

/// Today as `YYYY-MM-DD`, for a file's name.
pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// The timestamp of a `YYYY-MM-DD` day: midnight UTC, the way a browser reads a date-only text.
pub fn from_ymd(text: &str) -> Option<i64> {
    let day = NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()?;
    Some(Utc.from_utc_datetime(&day.and_hms_opt(0, 0, 0)?).timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_goes_to_a_timestamp_and_the_timestamp_names_the_day() {
        assert_eq!(from_ymd("1970-01-02"), Some(86_400_000));
        assert_eq!(from_ymd("not a date"), None);
        assert_eq!(from_ymd(""), None);
        // noon is the same calendar day in every time zone
        let noon = from_ymd("2026-03-05").unwrap() + 12 * 3_600_000;
        assert_eq!(ymd(noon), "2026-03-05");
    }
}
