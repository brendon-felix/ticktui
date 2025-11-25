use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    widgets::{Block, BorderType, Borders, Clear},
};
use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::Input;

use crate::{
    app::AppAction,
    taskparser::{TaskParser, TokenType},
    ui::{
        UIAction,
        editor::{
            Editor, EditorMode,
            actions::{EditorAction, EditorActions},
            handlers,
        },
        popup::Popup,
        utils,
    },
};

pub struct ReschedulePopup {
    editor: Editor,
    block: Block<'static>,
    tx: UnboundedSender<AppAction>,
}

impl ReschedulePopup {
    pub fn new(tx: UnboundedSender<AppAction>) -> Self {
        let mut editor = Editor::new().with_single_line();
        editor.set_mode(EditorMode::Insert);
        Self {
            editor,
            block: Block::new()
                .title(" Reschedule Tasks ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
            tx,
        }
    }

    fn submit(&mut self) {
        let content = self.editor.get_content().trim().to_string();
        if !content.is_empty() {
            let _ = self
                .tx
                .send(AppAction::UIAction(UIAction::RescheduleTask(content)));
            let _ = self.tx.send(AppAction::UIAction(UIAction::ClosePopup));
        }
    }
}

impl Popup for ReschedulePopup {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => {
                let _ = self.tx.send(AppAction::UIAction(UIAction::ClosePopup));
            }
            _ => {
                let input: Input = key_event.into();
                let mode = self.editor.get_mode();
                let action_opt = if let Some(pending_action) = self.editor.get_pending_action() {
                    match handlers::handle_pending_action_input(input, pending_action) {
                        Some(action) => Some(action),
                        None => {
                            self.editor.set_pending_action(None);
                            None
                        }
                    }
                } else {
                    handlers::handle_input(input, mode, true)
                };
                if let Some(action) = action_opt {
                    match action {
                        EditorAction::Submit => self.submit(),
                        _ => self.editor.execute_action(action),
                    }
                }
            }
        }
    }

    fn handle_mouse_event(&mut self, _mouse_event: MouseEvent) {}

    fn allow_key_cmd(&self) -> bool {
        self.editor.get_mode() == EditorMode::Normal
    }

    fn draw(&mut self, f: &mut Frame, area: Rect, _last_frame: Instant) {
        // let popup_area = utils::centered_area_with_offset(area, 3, 60, 4);
        let popup_area = utils::centered_area(area, 3, 60);
        f.render_widget(Clear, popup_area);

        let inner_area = self.block.inner(popup_area).inner(Margin::new(1, 0));
        f.render_widget(&self.block, popup_area);
        self.editor.update_style();
        f.render_widget(&self.editor, inner_area);
    }
}

#[derive(Debug)]
pub enum RescheduleTarget {
    /// Relative to the task's original due datetime (e.g., "5min", "2 hours")
    RelativeToDueDate(chrono::Duration),
    /// Absolute time from now (e.g., "now", "now + 5min")
    AbsoluteTime(chrono::DateTime<chrono::Local>),
}

pub fn parse_duration(duration_str: &str) -> Result<RescheduleTarget, String> {
    // First, try the legacy relative/absolute duration parsing for backward compatibility
    if let Ok(target) = parse_legacy_duration(duration_str) {
        return Ok(target);
    }

    // If legacy parsing fails, try full datetime parsing using TaskParser
    parse_datetime_with_taskparser(duration_str)
}

fn parse_legacy_duration(duration_str: &str) -> Result<RescheduleTarget, String> {
    let duration_str = duration_str.trim().to_lowercase();
    let is_absolute = duration_str.starts_with("now");
    if duration_str == "now" {
        return Ok(RescheduleTarget::AbsoluteTime(chrono::Local::now()));
    }

    let duration_part = if is_absolute {
        let after_now = duration_str.strip_prefix("now").unwrap().trim();
        after_now.strip_prefix("+").unwrap_or(after_now).trim()
    } else {
        &duration_str
    };

    let mut num_end = 0;
    for (i, ch) in duration_part.chars().enumerate() {
        if ch.is_ascii_digit() {
            num_end = i + 1;
        } else {
            break;
        }
    }

    if num_end == 0 {
        return Err(format!(
            "Invalid legacy duration format. Expected number followed by unit (e.g., '5min', '2 hours')"
        ));
    }

    let num_str = &duration_part[..num_end];
    let unit_str = duration_part[num_end..].trim();

    let value = num_str
        .parse::<i64>()
        .map_err(|_| format!("Invalid duration value: {}", num_str))?;

    let duration = match unit_str {
        "min" | "minute" | "minutes" => chrono::Duration::minutes(value),
        "hr" | "hour" | "hours" | "h" => chrono::Duration::hours(value),
        "day" | "days" | "d" => chrono::Duration::days(value),
        _ => {
            return Err(format!(
                "Invalid duration unit: '{}'. Use 'min', 'minutes', 'hr', 'hours', 'day', or 'days'",
                unit_str
            ));
        }
    };

    if is_absolute {
        Ok(RescheduleTarget::AbsoluteTime(
            chrono::Local::now() + duration,
        ))
    } else {
        Ok(RescheduleTarget::RelativeToDueDate(duration))
    }
}

fn parse_datetime_with_taskparser(input: &str) -> Result<RescheduleTarget, String> {
    let parser = TaskParser::new();
    let tokens = parser.parse(input);

    for (token, _text) in tokens {
        match token {
            TokenType::DueDate(dt) => {
                return Ok(RescheduleTarget::AbsoluteTime(dt));
            }
            _ => continue,
        }
    }

    // If no date was parsed, return an error
    Err(format!(
        "Could not parse '{}' as a valid date/time or duration. \
        Supported formats include: \
        - Legacy: '5min', '2 hours', 'now', 'now + 30min' \
        - Absolute dates: 'tomorrow', 'dec 12', 'next friday', '11/23' \
        - Times: '4pm', 'tomorrow 5pm', 'dec 12 3:30pm' \
        - And many other natural language date formats",
        input
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_duration_parsing() {
        // Test "now"
        match parse_duration("now").unwrap() {
            RescheduleTarget::AbsoluteTime(_) => (),
            _ => panic!("Expected AbsoluteTime for 'now'"),
        }

        // Test relative durations
        match parse_duration("5min").unwrap() {
            RescheduleTarget::RelativeToDueDate(duration) => {
                assert_eq!(duration.num_minutes(), 5);
            }
            _ => panic!("Expected RelativeToDueDate for '5min'"),
        }

        match parse_duration("2 hours").unwrap() {
            RescheduleTarget::RelativeToDueDate(duration) => {
                assert_eq!(duration.num_hours(), 2);
            }
            _ => panic!("Expected RelativeToDueDate for '2 hours'"),
        }

        // Test "now + duration"
        match parse_duration("now + 10min").unwrap() {
            RescheduleTarget::AbsoluteTime(_) => (),
            _ => panic!("Expected AbsoluteTime for 'now + 10min'"),
        }
    }

    #[test]
    fn test_taskparser_datetime_parsing() {
        // Test relative dates
        match parse_duration("tomorrow").unwrap() {
            RescheduleTarget::AbsoluteTime(_) => (),
            _ => panic!("Expected AbsoluteTime for 'tomorrow'"),
        }

        // Test times
        match parse_duration("4pm").unwrap() {
            RescheduleTarget::AbsoluteTime(_) => (),
            _ => panic!("Expected AbsoluteTime for '4pm'"),
        }

        // Test month/day
        match parse_duration("dec 25").unwrap() {
            RescheduleTarget::AbsoluteTime(_) => (),
            _ => panic!("Expected AbsoluteTime for 'dec 25'"),
        }

        // Test combined
        match parse_duration("tomorrow 5pm").unwrap() {
            RescheduleTarget::AbsoluteTime(_) => (),
            _ => panic!("Expected AbsoluteTime for 'tomorrow 5pm'"),
        }
    }

    #[test]
    fn test_invalid_input() {
        assert!(parse_duration("invalid input").is_err());
        assert!(parse_duration("").is_err());
        assert!(parse_duration("xyz123").is_err());
    }

    #[test]
    fn test_legacy_takes_precedence() {
        // Ensure legacy parsing is tried first for backward compatibility
        // "now" should be parsed as legacy absolute time, not as a title token
        match parse_duration("now").unwrap() {
            RescheduleTarget::AbsoluteTime(_) => (),
            _ => panic!("Legacy 'now' should be parsed as AbsoluteTime"),
        }
    }

    #[test]
    fn test_comprehensive_datetime_formats() {
        // Test all supported datetime formats to demonstrate functionality

        // Legacy relative formats (relative to task's due date)
        assert!(matches!(
            parse_duration("5min"),
            Ok(RescheduleTarget::RelativeToDueDate(_))
        ));
        assert!(matches!(
            parse_duration("2 hours"),
            Ok(RescheduleTarget::RelativeToDueDate(_))
        ));
        assert!(matches!(
            parse_duration("3 days"),
            Ok(RescheduleTarget::RelativeToDueDate(_))
        ));

        // Legacy absolute formats (absolute time from now)
        assert!(matches!(
            parse_duration("now"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));
        assert!(matches!(
            parse_duration("now + 5min"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));
        assert!(matches!(
            parse_duration("now + 2 hours"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));

        // Natural language dates
        assert!(matches!(
            parse_duration("today"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));
        assert!(matches!(
            parse_duration("tomorrow"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));
        assert!(matches!(
            parse_duration("next friday"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));
        assert!(matches!(
            parse_duration("friday"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));

        // Month and day formats
        assert!(matches!(
            parse_duration("dec 12"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));
        assert!(matches!(
            parse_duration("december 8"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));
        assert!(matches!(
            parse_duration("jan 1"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));

        // Slash date formats
        assert!(matches!(
            parse_duration("11/23"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));
        assert!(matches!(
            parse_duration("12/31/2024"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));

        // Time formats
        assert!(matches!(
            parse_duration("4pm"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));
        assert!(matches!(
            parse_duration("3:30pm"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));
        assert!(matches!(
            parse_duration("15:30"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));

        // Combined date and time formats
        assert!(matches!(
            parse_duration("tomorrow 5pm"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));
        assert!(matches!(
            parse_duration("dec 12 3:30pm"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));
        assert!(matches!(
            parse_duration("next friday 4pm"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));
        assert!(matches!(
            parse_duration("11/23 2pm"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));
        assert!(matches!(
            parse_duration("today 9am"),
            Ok(RescheduleTarget::AbsoluteTime(_))
        ));

        // Verify error cases still work
        assert!(parse_duration("invalid").is_err());
        assert!(parse_duration("xyz123abc").is_err());
        assert!(parse_duration("").is_err());
    }
}
