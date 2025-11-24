use crate::tasks::RepeatDay;
use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveTime};

/// Normalize string for case-insensitive matching
pub fn normalize_str(s: &str) -> String {
    s.to_lowercase().trim().to_string()
}

/// Match a string against multiple possible values (case-insensitive)
pub fn matches_any(s: &str, candidates: &[&str]) -> bool {
    let s_norm = normalize_str(s);
    candidates.iter().any(|&candidate| s_norm == candidate)
}

/// Parse weekday name to chrono::Weekday
pub fn parse_chrono_weekday(s: &str) -> Option<chrono::Weekday> {
    match normalize_str(s).as_str() {
        s if matches_any(s, &["monday", "mon"]) => Some(chrono::Weekday::Mon),
        s if matches_any(s, &["tuesday", "tue"]) => Some(chrono::Weekday::Tue),
        s if matches_any(s, &["wednesday", "wed"]) => Some(chrono::Weekday::Wed),
        s if matches_any(s, &["thursday", "thu", "thur"]) => Some(chrono::Weekday::Thu),
        s if matches_any(s, &["friday", "fri"]) => Some(chrono::Weekday::Fri),
        s if matches_any(s, &["saturday", "sat"]) => Some(chrono::Weekday::Sat),
        s if matches_any(s, &["sunday", "sun"]) => Some(chrono::Weekday::Sun),
        _ => None,
    }
}

/// Parse weekday name to RepeatDay
pub fn parse_repeat_weekday(s: &str) -> Option<RepeatDay> {
    match normalize_str(s).as_str() {
        s if matches_any(s, &["monday", "mon"]) => Some(RepeatDay::Monday),
        s if matches_any(s, &["tuesday", "tue"]) => Some(RepeatDay::Tuesday),
        s if matches_any(s, &["wednesday", "wed"]) => Some(RepeatDay::Wednesday),
        s if matches_any(s, &["thursday", "thu", "thur"]) => Some(RepeatDay::Thursday),
        s if matches_any(s, &["friday", "fri"]) => Some(RepeatDay::Friday),
        s if matches_any(s, &["saturday", "sat"]) => Some(RepeatDay::Saturday),
        s if matches_any(s, &["sunday", "sun"]) => Some(RepeatDay::Sunday),
        _ => None,
    }
}

/// Parse month name to month number (1-12)
pub fn parse_month_name(s: &str) -> Option<u32> {
    match normalize_str(s).as_str() {
        s if matches_any(s, &["jan", "january"]) => Some(1),
        s if matches_any(s, &["feb", "february"]) => Some(2),
        s if matches_any(s, &["mar", "march"]) => Some(3),
        s if matches_any(s, &["apr", "april"]) => Some(4),
        s if matches_any(s, &["may"]) => Some(5),
        s if matches_any(s, &["jun", "june"]) => Some(6),
        s if matches_any(s, &["jul", "july"]) => Some(7),
        s if matches_any(s, &["aug", "august"]) => Some(8),
        s if matches_any(s, &["sep", "september"]) => Some(9),
        s if matches_any(s, &["oct", "october"]) => Some(10),
        s if matches_any(s, &["nov", "november"]) => Some(11),
        s if matches_any(s, &["dec", "december"]) => Some(12),
        _ => None,
    }
}

/// Parse day number with ordinal suffixes (1st, 2nd, 3rd, 4th, etc.)
pub fn parse_day_number(s: &str) -> Option<u32> {
    let s_clean = s
        .trim_end_matches("st")
        .trim_end_matches("nd")
        .trim_end_matches("rd")
        .trim_end_matches("th");
    s_clean.parse().ok()
}

/// Calculate next occurrence of a weekday (always in the future)
pub fn next_weekday(weekday: chrono::Weekday) -> DateTime<Local> {
    let now = Local::now();
    let current_weekday = now.weekday();
    let days_until = ((weekday.num_days_from_monday() as i64
        - current_weekday.num_days_from_monday() as i64
        + 7)
        % 7) as i64;
    let days_until = if days_until == 0 { 7 } else { days_until };

    (now + chrono::Duration::days(days_until))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap()
}

/// Calculate next occurrence of a weekday within the current week (could be today)
pub fn next_weekday_within_week(weekday: chrono::Weekday) -> DateTime<Local> {
    let now = Local::now();
    let current_weekday = now.weekday();
    let days_until = ((weekday.num_days_from_monday() as i64
        - current_weekday.num_days_from_monday() as i64
        + 7)
        % 7) as i64;

    (now + chrono::Duration::days(days_until))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap()
}

/// Create a date with time from components
pub fn create_date_with_time(date: NaiveDate, time: Option<NaiveTime>) -> DateTime<Local> {
    let final_time = time.unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    date.and_time(final_time).and_local_timezone(Local).unwrap()
}
