use chrono::{DateTime, Local, Utc};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn format_date(dt: &DateTime<Utc>, is_all_day: bool, is_today: bool) -> Option<String> {
    if dt.timestamp() == 0 {
        None
    } else {
        let local: DateTime<Local> = dt.with_timezone(&Local);
        match (is_today, is_all_day) {
            (true, true) => Some("Today".to_string()),
            (true, false) => Some(local.format("Today %I:%M %p").to_string()),
            (false, true) => Some(local.format("%m/%d/%Y").to_string()),
            (false, false) => Some(local.format("%m/%d/%Y %I:%M %p").to_string()),
        }
    }
}

pub fn centered_area(area: Rect, height: u16, width: u16) -> Rect {
    Layout::new(
        Direction::Horizontal,
        [
            Constraint::Fill(1),
            Constraint::Length(width),
            Constraint::Fill(1),
        ],
    )
    .split(
        Layout::new(
            Direction::Vertical,
            [
                Constraint::Fill(1),
                Constraint::Length(height),
                Constraint::Fill(1),
            ],
        )
        .split(area)[1],
    )[1]
}
