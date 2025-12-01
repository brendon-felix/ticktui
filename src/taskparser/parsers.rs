use chrono::{Datelike, Local, NaiveDate};
use ticks::tasks::TaskPriority;

use crate::{
    taskparser::{
        TokenParser, TokenType, parseutils, starts_with_any, timeutils, try_parse_pattern,
    },
    tasks::{RepeatFlag, RepeatFreq},
};

pub struct DueDateParser;

impl DueDateParser {
    fn parse_relative_dates(&self, words: &[&str]) -> Option<(TokenType, usize)> {
        if words.is_empty() {
            return None;
        }

        let now = Local::now();
        let today = now.date_naive();

        // Try time-first patterns (e.g., "3pm today", "4pm tomorrow")
        if let Some(time) = parseutils::parse_time(words[0]) {
            if words.len() > 1 {
                match parseutils::normalize_str(words[1]).as_str() {
                    "today" => {
                        let dt = timeutils::create_date_with_time(today, Some(time));
                        return Some((TokenType::DueDate(dt), 2));
                    }
                    "tomorrow" => {
                        let tomorrow = today + chrono::Duration::days(1);
                        let dt = timeutils::create_date_with_time(tomorrow, Some(time));
                        return Some((TokenType::DueDate(dt), 2));
                    }
                    _ => {}
                }
            }
        }

        // Try date-first patterns (e.g., "today", "today 3pm")
        match parseutils::normalize_str(words[0]).as_str() {
            "today" => {
                let time = if words.len() > 1 {
                    parseutils::parse_time(words[1])
                } else {
                    None
                };
                let dt = timeutils::create_date_with_time(today, time);
                Some((TokenType::DueDate(dt), if time.is_some() { 2 } else { 1 }))
            }
            "tomorrow" => {
                let tomorrow = today + chrono::Duration::days(1);
                let time = if words.len() > 1 {
                    parseutils::parse_time(words[1])
                } else {
                    None
                };
                let dt = timeutils::create_date_with_time(tomorrow, time);
                Some((TokenType::DueDate(dt), if time.is_some() { 2 } else { 1 }))
            }
            _ => None,
        }
    }

    fn parse_next_weekday(&self, words: &[&str]) -> Option<(TokenType, usize)> {
        if words.len() < 2 {
            return None;
        }

        // Try time-first pattern (e.g., "3pm next friday")
        if let Some(time) = parseutils::parse_time(words[0]) {
            if words.len() > 2 && parseutils::normalize_str(words[1]) == "next" {
                if let Some(weekday) = parseutils::parse_chrono_weekday(words[2]) {
                    let base_dt = timeutils::next_weekday(weekday);
                    let dt = base_dt
                        .date_naive()
                        .and_time(time)
                        .and_local_timezone(Local)
                        .unwrap();
                    return Some((TokenType::DueDate(dt), 3));
                }
            }
        }

        // Try date-first pattern (e.g., "next friday")
        try_parse_pattern(words, 2, |words| {
            if parseutils::normalize_str(words[0]) == "next" {
                parseutils::parse_chrono_weekday(words[1])
                    .map(|weekday| TokenType::DueDate(timeutils::next_weekday(weekday)))
            } else {
                None
            }
        })
    }

    fn parse_standalone_weekday(&self, words: &[&str]) -> Option<(TokenType, usize)> {
        if words.is_empty() {
            return None;
        }

        // Try time-first pattern (e.g., "3pm friday")
        if let Some(time) = parseutils::parse_time(words[0]) {
            if words.len() > 1 {
                if let Some(weekday) = parseutils::parse_chrono_weekday(words[1]) {
                    let base_dt = timeutils::next_weekday_within_week(weekday);
                    let dt = base_dt
                        .date_naive()
                        .and_time(time)
                        .and_local_timezone(Local)
                        .unwrap();
                    return Some((TokenType::DueDate(dt), 2));
                }
            }
        }

        // Try date-first pattern (e.g., "friday", "friday 3pm")
        if let Some(weekday) = parseutils::parse_chrono_weekday(words[0]) {
            let time = if words.len() > 1 {
                parseutils::parse_time(words[1])
            } else {
                None
            };
            let base_dt = timeutils::next_weekday_within_week(weekday);
            let dt = if let Some(time) = time {
                base_dt
                    .date_naive()
                    .and_time(time)
                    .and_local_timezone(Local)
                    .unwrap()
            } else {
                base_dt
            };
            Some((TokenType::DueDate(dt), if time.is_some() { 2 } else { 1 }))
        } else {
            None
        }
    }

    fn parse_time_only(&self, words: &[&str]) -> Option<(TokenType, usize)> {
        parseutils::parse_time(words[0]).map(|time| {
            let today = Local::now().date_naive();
            let dt = timeutils::create_date_with_time(today, Some(time));
            (TokenType::DueDate(dt), 1)
        })
    }

    fn parse_month_day(&self, words: &[&str]) -> Option<(TokenType, usize)> {
        if words.len() < 2 {
            return None;
        }

        // Try time-first pattern (e.g., "3pm dec 3")
        if let Some(time) = parseutils::parse_time(words[0]) {
            if words.len() > 2 {
                if let (Some(month), Some(day)) = (
                    parseutils::parse_month_name(words[1]),
                    parseutils::parse_day_number(words[2]),
                ) {
                    let year = Local::now().year();
                    if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                        let dt = timeutils::create_date_with_time(date, Some(time));
                        return Some((TokenType::DueDate(dt), 3));
                    }
                }
            }
        }

        // Try date-first pattern (e.g., "dec 3", "dec 3 3pm")
        if let (Some(month), Some(day)) = (
            parseutils::parse_month_name(words[0]),
            parseutils::parse_day_number(words[1]),
        ) {
            let year = Local::now().year();
            if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                // Check if there's a time component in the third word
                let time = if words.len() > 2 {
                    parseutils::parse_time(words[2])
                } else {
                    None
                };
                let dt = timeutils::create_date_with_time(date, time);
                let consumed = if time.is_some() { 3 } else { 2 };
                Some((TokenType::DueDate(dt), consumed))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn parse_slash_date(&self, words: &[&str]) -> Option<(TokenType, usize)> {
        if words.is_empty() {
            return None;
        }

        // Try time-first pattern (e.g., "3pm 12/31")
        if let Some(time) = parseutils::parse_time(words[0]) {
            if words.len() > 1 && words[1].contains('/') {
                let parts: Vec<&str> = words[1].split('/').collect();
                let today = Local::now().date_naive();

                match parts.len() {
                    3 => {
                        // Format: MM/DD/YYYY
                        if let (Ok(month), Ok(day), Ok(year)) = (
                            parts[0].parse::<u32>(),
                            parts[1].parse::<u32>(),
                            parts[2].parse::<i32>(),
                        ) {
                            if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                                let dt = timeutils::create_date_with_time(date, Some(time));
                                return Some((TokenType::DueDate(dt), 2));
                            }
                        }
                    }
                    2 => {
                        // Format: MM/DD (current year)
                        if let (Ok(month), Ok(day)) =
                            (parts[0].parse::<u32>(), parts[1].parse::<u32>())
                        {
                            let year = today.year();
                            if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                                let dt = timeutils::create_date_with_time(date, Some(time));
                                return Some((TokenType::DueDate(dt), 2));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Try date-first pattern (e.g., "12/31", "12/31/2024")
        if !words[0].contains('/') {
            return None;
        }

        let parts: Vec<&str> = words[0].split('/').collect();
        let today = Local::now().date_naive();

        match parts.len() {
            3 => {
                // Format: MM/DD/YYYY
                if let (Ok(month), Ok(day), Ok(year)) = (
                    parts[0].parse::<u32>(),
                    parts[1].parse::<u32>(),
                    parts[2].parse::<i32>(),
                ) {
                    NaiveDate::from_ymd_opt(year, month, day).map(|date| {
                        (
                            TokenType::DueDate(timeutils::create_date_with_time(date, None)),
                            1,
                        )
                    })
                } else {
                    None
                }
            }
            2 => {
                // Format: MM/DD (current year)
                if let (Ok(month), Ok(day)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    let year = today.year();
                    NaiveDate::from_ymd_opt(year, month, day).map(|date| {
                        (
                            TokenType::DueDate(timeutils::create_date_with_time(date, None)),
                            1,
                        )
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl TokenParser for DueDateParser {
    fn parse(&self, words: &[&str]) -> Option<(TokenType, usize)> {
        if words.is_empty() {
            return None;
        }

        // Try different parsing strategies in order of preference
        self.parse_relative_dates(words)
            .or_else(|| self.parse_next_weekday(words))
            .or_else(|| self.parse_standalone_weekday(words))
            .or_else(|| self.parse_time_only(words))
            .or_else(|| self.parse_month_day(words))
            .or_else(|| self.parse_slash_date(words))
    }
}

pub struct PriorityParser;

impl PriorityParser {
    fn parse_exclamation_priority(&self, word: &str) -> Option<TaskPriority> {
        if !word.starts_with('!') {
            return None;
        }

        let priority_str = &word[1..].to_lowercase();
        match priority_str {
            s if parseutils::matches_any(s, &["high", "h"]) => Some(TaskPriority::High),
            s if parseutils::matches_any(s, &["medium", "med", "m"]) => Some(TaskPriority::Medium),
            s if parseutils::matches_any(s, &["low", "l"]) => Some(TaskPriority::Low),
            _ => None,
        }
    }

    fn parse_p_priority(&self, word: &str) -> Option<TaskPriority> {
        let word_lower = word.to_lowercase();
        if word_lower.starts_with('p') && word.len() == 2 {
            match &word_lower[1..] {
                "1" => Some(TaskPriority::High),
                "2" => Some(TaskPriority::Medium),
                "3" => Some(TaskPriority::Low),
                _ => None,
            }
        } else {
            None
        }
    }
}

impl TokenParser for PriorityParser {
    fn parse(&self, words: &[&str]) -> Option<(TokenType, usize)> {
        if words.is_empty() {
            return None;
        }

        let word = words[0];

        self.parse_exclamation_priority(word)
            .or_else(|| self.parse_p_priority(word))
            .map(|priority| (TokenType::Priority(priority), 1))
    }
}

pub struct RepeatParser;

impl RepeatParser {
    fn parse_simple_frequencies(&self, word: &str) -> Option<RepeatFlag> {
        match parseutils::normalize_str(word).as_str() {
            "daily" => Some(RepeatFlag::new(RepeatFreq::Daily, 1, None)),
            "weekly" => Some(RepeatFlag::new(RepeatFreq::Weekly, 1, None)),
            "monthly" => Some(RepeatFlag::new(RepeatFreq::Monthly, 1, None)),
            "yearly" => Some(RepeatFlag::new(RepeatFreq::Yearly, 1, None)),
            _ => None,
        }
    }

    fn parse_every_patterns(&self, words: &[&str]) -> Option<(RepeatFlag, usize)> {
        if words.is_empty() || parseutils::normalize_str(words[0]) != "every" {
            return None;
        }

        if words.len() < 2 {
            return None;
        }

        let second_word = parseutils::normalize_str(&words[1]);

        // Handle simple "every X" patterns
        let simple_patterns = [
            ("weekday", RepeatFlag::new(RepeatFreq::Weekdays, 1, None)),
            ("weekdays", RepeatFlag::new(RepeatFreq::Weekdays, 1, None)),
            ("day", RepeatFlag::new(RepeatFreq::Daily, 1, None)),
            ("week", RepeatFlag::new(RepeatFreq::Weekly, 1, None)),
            ("month", RepeatFlag::new(RepeatFreq::Monthly, 1, None)),
            ("year", RepeatFlag::new(RepeatFreq::Yearly, 1, None)),
        ];

        for (pattern, flag) in &simple_patterns {
            if second_word == *pattern {
                return Some((flag.clone(), 2));
            }
        }

        // Handle "every weekday" patterns
        if let Some(day) = parseutils::parse_repeat_weekday(&second_word) {
            let flag = RepeatFlag::new(RepeatFreq::Weekly, 1, Some(vec![day]));
            return Some((flag, 2));
        }

        // Handle "every other X" patterns
        if words.len() >= 3 && second_word == "other" {
            let third_word = parseutils::normalize_str(&words[2]);
            match third_word.as_str() {
                "day" => Some((RepeatFlag::new(RepeatFreq::Daily, 2, None), 3)),
                "week" => Some((RepeatFlag::new(RepeatFreq::Weekly, 2, None), 3)),
                _ => None,
            }
        } else if words.len() >= 3 {
            // Handle "every month day" patterns (yearly repetition)
            Some((RepeatFlag::new(RepeatFreq::Yearly, 1, None), 3))
        } else {
            None
        }
    }
}

impl TokenParser for RepeatParser {
    fn parse(&self, words: &[&str]) -> Option<(TokenType, usize)> {
        if words.is_empty() {
            return None;
        }

        // Try simple frequency words first
        if let Some(flag) = self.parse_simple_frequencies(words[0]) {
            return Some((TokenType::Repeat(flag), 1));
        }

        // Try "every..." patterns
        if let Some((flag, consumed)) = self.parse_every_patterns(words) {
            return Some((TokenType::Repeat(flag), consumed));
        }

        None
    }
}

pub struct ProjectNameParser;

impl TokenParser for ProjectNameParser {
    fn parse(&self, words: &[&str]) -> Option<(TokenType, usize)> {
        if words.is_empty() {
            return None;
        }

        let word = words[0];
        let project_prefixes = ['^', '#', '~'];

        if let Some(_prefix) = starts_with_any(word, &project_prefixes) {
            let project_name = word[1..].to_string();
            if !project_name.is_empty() {
                return Some((TokenType::ProjectName(project_name), 1));
            }
        }

        None
    }
}
