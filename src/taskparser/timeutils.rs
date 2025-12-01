use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, Timelike, Weekday};

use crate::tasks::{RepeatFlag, RepeatFreq};

pub fn create_12hour_time(hour: u32, minute: u32, is_pm: bool) -> Option<NaiveTime> {
    let mut h = hour;
    if is_pm && h != 12 {
        h += 12;
    } else if !is_pm && h == 12 {
        h = 0;
    }
    NaiveTime::from_hms_opt(h, minute, 0)
}

pub fn get_day_start(dt: DateTime<Local>) -> DateTime<Local> {
    dt.date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap()
}

pub fn initial_occurrence(repeat_flag: &RepeatFlag) -> Option<DateTime<Local>> {
    match repeat_flag.freq() {
        RepeatFreq::Weekly => {
            if let Some(days) = repeat_flag.days() {
                if !days.is_empty() {
                    let weekday = days[0];
                    Some(next_weekday_within_week(weekday))
                } else {
                    None
                }
            } else {
                None
            }
        }
        RepeatFreq::Weekdays => {
            let now = Local::now();
            let current_weekday = now.weekday();

            let weekdays = [
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri,
            ];

            for &weekday in &weekdays {
                let days_until = ((weekday.num_days_from_monday() as i64
                    - current_weekday.num_days_from_monday() as i64
                    + 7)
                    % 7) as i64;

                if days_until > 0 || (days_until == 0 && now.hour() < 9) {
                    let day_start = get_day_start(
                        now + Duration::days(if days_until == 0 { 0 } else { days_until }),
                    );
                    return Some(day_start);
                }
            }

            Some(next_weekday(chrono::Weekday::Mon))
        }
        RepeatFreq::Daily => {
            let now = Local::now();
            let day_start = get_day_start(now + Duration::days(1));
            Some(day_start)
        }
        _ => None,
    }
}

/// Create DateTime<Local> from NaiveDate and optional NaiveTime (defaults to 00:00)
pub fn create_date_with_time(date: NaiveDate, time: Option<NaiveTime>) -> DateTime<Local> {
    let final_time = time.unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    date.and_time(final_time).and_local_timezone(Local).unwrap()
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

    get_day_start(now + Duration::days(days_until))
}

/// Calculate next occurrence of a weekday within the current week (could be today)
pub fn next_weekday_within_week(weekday: Weekday) -> DateTime<Local> {
    let now = Local::now();
    let current_weekday = now.weekday();
    let days_until = ((weekday.num_days_from_monday() as i64
        - current_weekday.num_days_from_monday() as i64
        + 7)
        % 7) as i64;

    get_day_start(now + Duration::days(days_until))
}
