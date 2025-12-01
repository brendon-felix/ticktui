pub mod parsers;
pub mod parseutils;
pub mod timeutils;

use chrono::{DateTime, Local};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ticks::tasks::TaskPriority;

use crate::taskparser::parsers::{DueDateParser, PriorityParser, ProjectNameParser, RepeatParser};
use crate::tasks::RepeatFlag;

#[derive(Debug, Clone)]
pub enum TokenType {
    DueDate(DateTime<Local>), // "3pm", "today", "today 8:30", "Tomorrow 4:30PM", "next friday", "dec 20th", "12/31/2024"
    Priority(TaskPriority),   // "!High", "P1", "p1", "!low", "P3"
    ProjectName(String), // "^ProjectName", "#projectname/section", "~ProjectName", "~project/section"
    Repeat(RepeatFlag), // "every monday", "daily", "weekly", "every dec 20th", "every weekday", "every other day"
    Title(String),      // any unmatched word
}

trait TokenParser {
    fn parse(&self, words: &[&str]) -> Option<(TokenType, usize)>;
}

pub struct TaskParser {
    parsers: Vec<Box<dyn TokenParser>>,
}

impl TaskParser {
    pub fn new() -> Self {
        Self {
            parsers: vec![
                Box::new(DueDateParser),
                Box::new(PriorityParser),
                Box::new(RepeatParser),
                Box::new(ProjectNameParser),
            ],
        }
    }

    pub fn parse(&self, input: &str) -> Vec<(TokenType, String)> {
        let words: Vec<&str> = input.split_whitespace().collect();
        let mut tokens = Vec::new();
        let mut i = 0;

        while i < words.len() {
            let remaining = &words[i..];
            let mut matched = false;

            // Try each parser in order
            for parser in &self.parsers {
                if let Some((token, consumed)) = parser.parse(remaining) {
                    // Store the token and the original text
                    let original_text = remaining[..consumed].join(" ");
                    tokens.push((token, original_text));
                    i += consumed;
                    matched = true;
                    break;
                }
            }

            // If no parser matched, treat as title
            if !matched {
                tokens.push((TokenType::Title(words[i].to_string()), words[i].to_string()));
                i += 1;
            }
        }

        tokens
    }

    pub fn highlighted_spans(&self, input: &str) -> Vec<Span<'_>> {
        let tokens = self.parse(input);
        let mut spans = Vec::new();

        for (i, (token, text)) in tokens.iter().enumerate() {
            let span = create_styled_span(token, text);
            spans.push(span);

            // Add space between tokens (except after the last one)
            if i < tokens.len() - 1 {
                spans.push(Span::raw(" "));
            }
        }

        spans
    }
}

pub fn try_parse_pattern<T, F>(
    words: &[&str],
    expected_count: usize,
    parser: F,
) -> Option<(T, usize)>
where
    F: FnOnce(&[&str]) -> Option<T>,
{
    if words.len() >= expected_count {
        parser(&words[..expected_count]).map(|result| (result, expected_count))
    } else {
        None
    }
}

/// Check if word starts with any of the given prefixes
pub fn starts_with_any(word: &str, prefixes: &[char]) -> Option<char> {
    prefixes
        .iter()
        .find(|&&prefix| word.starts_with(prefix))
        .copied()
}

// Styling utilities
fn create_styled_span(token: &TokenType, text: &str) -> Span<'static> {
    match token {
        TokenType::DueDate(_) => Span::styled(text.to_string(), Style::default().fg(Color::Cyan)),
        TokenType::Priority(p) => {
            let color = match p {
                TaskPriority::High => Color::Red,
                TaskPriority::Medium => Color::Yellow,
                TaskPriority::Low => Color::Blue,
                TaskPriority::None => Color::Gray,
            };
            Span::styled(text.to_string(), Style::default().fg(color))
        }
        TokenType::ProjectName(_) => {
            Span::styled(text.to_string(), Style::default().fg(Color::Magenta))
        }
        TokenType::Repeat(_) => {
            Span::styled(text.to_string(), Style::default().fg(Color::LightYellow))
        }
        TokenType::Title(_) => Span::styled(text.to_string(), Style::default()),
    }
}
