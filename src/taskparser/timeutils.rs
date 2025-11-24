use chrono::NaiveTime;

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

    // Try simple hour format like "3pm"
    if let Ok(hour) = time_part.parse::<u32>() {
        return create_12hour_time(hour, 0, is_pm);
    }

    // Try "3:30pm" format
    if let Some((hour_str, min_str)) = time_part.split_once(':') {
        if let (Ok(hour), Ok(min)) = (hour_str.parse::<u32>(), min_str.parse::<u32>()) {
            return create_12hour_time(hour, min, is_pm);
        }
    }

    None
}

fn create_12hour_time(hour: u32, minute: u32, is_pm: bool) -> Option<NaiveTime> {
    let mut h = hour;
    if is_pm && h != 12 {
        h += 12;
    } else if !is_pm && h == 12 {
        h = 0;
    }
    NaiveTime::from_hms_opt(h, minute, 0)
}

fn parse_24hour_time(s: &str) -> Option<NaiveTime> {
    if let Some((hour_str, min_str)) = s.split_once(':') {
        if let (Ok(hour), Ok(min)) = (hour_str.parse::<u32>(), min_str.parse::<u32>()) {
            return NaiveTime::from_hms_opt(hour, min, 0);
        }
    }
    None
}
