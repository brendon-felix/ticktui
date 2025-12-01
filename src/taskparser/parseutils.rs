use chrono::{NaiveTime, Weekday};

use crate::taskparser::timeutils;

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
        s if matches_any(s, &["monday", "mon"]) => Some(Weekday::Mon),
        s if matches_any(s, &["tuesday", "tue"]) => Some(Weekday::Tue),
        s if matches_any(s, &["wednesday", "wed"]) => Some(chrono::Weekday::Wed),
        s if matches_any(s, &["thursday", "thu", "thur"]) => Some(Weekday::Thu),
        s if matches_any(s, &["friday", "fri"]) => Some(Weekday::Fri),
        s if matches_any(s, &["saturday", "sat"]) => Some(Weekday::Sat),
        s if matches_any(s, &["sunday", "sun"]) => Some(Weekday::Sun),
        _ => None,
    }
}

/// Parse weekday name to Weekday
pub fn parse_repeat_weekday(s: &str) -> Option<Weekday> {
    match normalize_str(s).as_str() {
        s if matches_any(s, &["monday", "mon"]) => Some(Weekday::Mon),
        s if matches_any(s, &["tuesday", "tue"]) => Some(Weekday::Tue),
        s if matches_any(s, &["wednesday", "wed"]) => Some(Weekday::Wed),
        s if matches_any(s, &["thursday", "thu", "thur"]) => Some(Weekday::Thu),
        s if matches_any(s, &["friday", "fri"]) => Some(Weekday::Fri),
        s if matches_any(s, &["saturday", "sat"]) => Some(Weekday::Sat),
        s if matches_any(s, &["sunday", "sun"]) => Some(Weekday::Sun),
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

pub fn parse_time(s: &str) -> Option<NaiveTime> {
    parse_12hour_time(s).or_else(|| parse_24hour_time(s))
}

fn parse_12hour_time(s: &str) -> Option<NaiveTime> {
    let s_lower = s.to_lowercase();
    let is_pm = s_lower.ends_with("pm");
    let is_am = s_lower.ends_with("am");

    if !is_pm && !is_am {
        return None;
    }

    let time_part = s_lower.trim_end_matches("pm").trim_end_matches("am");

    // like "3pm"
    if let Ok(hour) = time_part.parse::<u32>() {
        return timeutils::create_12hour_time(hour, 0, is_pm);
    }

    // like "3:30pm"
    if let Some((hour_str, min_str)) = time_part.split_once(':') {
        if let (Ok(hour), Ok(min)) = (hour_str.parse::<u32>(), min_str.parse::<u32>()) {
            return timeutils::create_12hour_time(hour, min, is_pm);
        }
    }

    None
}

fn parse_24hour_time(s: &str) -> Option<NaiveTime> {
    if let Some((hour_str, min_str)) = s.split_once(':') {
        if let (Ok(hour), Ok(min)) = (hour_str.parse::<u32>(), min_str.parse::<u32>()) {
            return NaiveTime::from_hms_opt(hour, min, 0);
        }
    }
    None
}
