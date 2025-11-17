use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{Frame, layout::Rect};

mod animate;
mod composite;
mod editor;
mod focuslist;
mod focusmode;
mod multiselect;
mod normalmode;
pub mod popup;
mod taskeditor;
mod tasklist;
mod utils;

use focusmode::FocusModeUI;
use normalmode::NormalModeUI;
use std::{sync::Arc, time::Instant};
use ticks::tasks::Task;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::AppAction,
    ui::popup::{ConfirmationPopup, Popup},
};

enum AppUIMode {
    Focus,
    Normal,
}

pub struct AppUI {
    mode: AppUIMode,
    focus_ui: FocusModeUI,
    normal_ui: NormalModeUI,
    popup: Option<Box<dyn Popup>>,
    tx: UnboundedSender<AppAction>,
}

impl AppUI {
    pub fn new(tx: UnboundedSender<AppAction>) -> Self {
        Self {
            mode: AppUIMode::Normal,
            focus_ui: FocusModeUI::new(tx.clone()),
            normal_ui: NormalModeUI::new(tx.clone()),
            popup: None,
            tx,
        }
    }

    pub fn confirm(&mut self, pending_action: AppAction) {
        let popup = ConfirmationPopup::new(pending_action, self.tx.clone());
        self.popup = Some(Box::new(popup));
    }

    pub fn close_popup(&mut self) {
        self.popup = None;
    }

    pub fn update_tasks(&mut self, tasks: Vec<Arc<Task>>) {
        self.focus_ui.update_tasks(tasks.clone());
        self.normal_ui.update_tasks(tasks);
    }

    pub fn allow_quit(&self) -> bool {
        match &self.popup {
            Some(p) => p.allow_quit(),
            None => match self.mode {
                AppUIMode::Focus => self.focus_ui.allow_quit(),
                AppUIMode::Normal => self.normal_ui.allow_quit(),
            },
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        // 'q' and 'ctrl+c' are handled by app.rs
        match &mut self.popup {
            Some(popup) => popup.handle_key_event(key_event),
            None => match key_event.code {
                KeyCode::F(1) => self.mode = AppUIMode::Focus,
                KeyCode::F(2) => self.mode = AppUIMode::Normal,
                _ => match self.mode {
                    AppUIMode::Focus => self.focus_ui.handle_key_event(key_event),
                    AppUIMode::Normal => self.normal_ui.handle_key_event(key_event),
                },
            },
        }
    }

    pub fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        match &mut self.popup {
            Some(popup) => popup.handle_mouse_event(mouse_event),
            None => match self.mode {
                AppUIMode::Focus => self.focus_ui.handle_mouse_event(mouse_event),
                AppUIMode::Normal => self.normal_ui.handle_mouse_event(mouse_event),
            },
        }
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect, last_frame: Instant) {
        match self.mode {
            AppUIMode::Focus => self.focus_ui.draw(f, area, last_frame),
            AppUIMode::Normal => self.normal_ui.draw(f, area, last_frame),
        }
        if let Some(popup) = &mut self.popup {
            popup.draw(f, area, last_frame);
        }
    }
}
