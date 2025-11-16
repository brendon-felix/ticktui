use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{Frame, layout::Rect};

mod animate;
mod composite;
mod editor;
mod focuslist;
mod focusmode;
mod multiselect;
mod normalmode;
mod taskeditor;
mod tasklist;

use focusmode::FocusModeUI;
use normalmode::NormalModeUI;
use std::{sync::Arc, time::Instant};
use ticks::tasks::Task;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::AppAction;

enum AppUIMode {
    Focus,
    Normal,
}

pub struct AppUI {
    mode: AppUIMode,
    focus_ui: FocusModeUI,
    normal_ui: NormalModeUI,
    // tx: UnboundedSender<AppAction>,
}

impl AppUI {
    pub fn new(tx: UnboundedSender<AppAction>) -> Self {
        Self {
            mode: AppUIMode::Normal,
            focus_ui: FocusModeUI::new(tx.clone()),
            normal_ui: NormalModeUI::new(tx),
            // tx,
        }
    }

    pub fn update_tasks(&mut self, tasks: Vec<Arc<Task>>) {
        self.focus_ui.update_tasks(tasks.clone());
        self.normal_ui.update_tasks(tasks);
    }

    pub fn is_in_insert_mode(&self) -> bool {
        match self.mode {
            AppUIMode::Focus => self.focus_ui.is_in_insert_mode(),
            AppUIMode::Normal => self.normal_ui.is_in_insert_mode(),
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        // 'q' and 'ctrl+c' are handled by app.rs
        match key_event.code {
            KeyCode::F(1) => self.mode = AppUIMode::Focus,
            KeyCode::F(2) => self.mode = AppUIMode::Normal,
            _ => match self.mode {
                AppUIMode::Focus => self.focus_ui.handle_key_event(key_event),
                AppUIMode::Normal => self.normal_ui.handle_key_event(key_event),
            },
        }
    }

    pub fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        match self.mode {
            AppUIMode::Focus => self.focus_ui.handle_mouse_event(mouse_event),
            AppUIMode::Normal => self.normal_ui.handle_mouse_event(mouse_event),
        }
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect, last_frame: Instant) {
        match self.mode {
            AppUIMode::Focus => self.focus_ui.draw(f, area, last_frame),
            AppUIMode::Normal => self.normal_ui.draw(f, area, last_frame),
        }
    }
}
