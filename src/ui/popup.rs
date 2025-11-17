use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    Frame,
    layout::{Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::app::AppAction;

pub trait Popup {
    fn handle_key_event(&mut self, key_event: KeyEvent);
    fn handle_mouse_event(&mut self, mouse_event: MouseEvent);
    fn allow_quit(&self) -> bool;
    fn draw(&mut self, f: &mut Frame, area: Rect, last_frame: Instant);
}

pub struct ConfirmationPopup {
    pending_action: AppAction,
    tx: UnboundedSender<AppAction>,
}

impl ConfirmationPopup {
    pub fn new(pending_action: AppAction, tx: UnboundedSender<AppAction>) -> Self {
        Self { pending_action, tx }
    }
}

impl Popup for ConfirmationPopup {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        // Handle key events for confirmation (e.g., Y/N)
        match key_event.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let _ = self.tx.send(self.pending_action.clone());
                let _ = self.tx.send(AppAction::ClosePopup);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                let _ = self.tx.send(AppAction::ClosePopup);
            }
            _ => {}
        }
    }

    fn handle_mouse_event(&mut self, _mouse_event: MouseEvent) {
        // Handle mouse events if necessary
    }

    fn allow_quit(&self) -> bool {
        false
    }

    fn draw(&mut self, f: &mut Frame, area: Rect, _last_frame: Instant) {
        let block = Block::default()
            .borders(Borders::ALL)
            // .title("Confirm Action")
            .border_type(BorderType::Thick)
            .style(Style::default().fg(Color::Red).bg(Color::Rgb(30, 30, 30)));
        // block.render(area, buf);

        let popup_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    ratatui::layout::Constraint::Fill(1),
                    ratatui::layout::Constraint::Length(9),
                    ratatui::layout::Constraint::Fill(1),
                ]
                .as_ref(),
            )
            .split(area)[1];
        let popup_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    ratatui::layout::Constraint::Fill(1),
                    ratatui::layout::Constraint::Length(50),
                    ratatui::layout::Constraint::Fill(1),
                ]
                .as_ref(),
            )
            .split(popup_area)[1];

        f.render_widget(Clear, popup_area);

        let inner_area = block.inner(popup_area);
        f.render_widget(block, popup_area);

        let paragraph = Paragraph::new("Are you sure you want to proceed? (Y/N)").centered();

        // paragraph.render(inner_area, buf);
        f.render_widget(paragraph, inner_area);
    }
}
