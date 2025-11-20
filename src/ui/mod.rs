use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{Frame, layout::Rect, text::Text, widgets::Paragraph};

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
mod views;

use focusmode::FocusModeUI;
use normalmode::NormalModeUI;
use std::{sync::Arc, time::Instant};
use ticks::tasks::Task;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::AppAction,
    ui::{
        popup::{Popup, confirm::ConfirmationPopup, debug::DebugPopup},
        views::View,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum AppUIMode {
    Focus,
    Normal,
}

pub struct AppUI {
    mode: AppUIMode,
    focus_ui: FocusModeUI,
    normal_ui: NormalModeUI,
    popup: Option<Box<dyn Popup>>,
    debug_popup: Option<DebugPopup>,
    tx: UnboundedSender<AppAction>,
}

impl AppUI {
    pub fn new(tx: UnboundedSender<AppAction>) -> Self {
        Self {
            mode: AppUIMode::Normal,
            focus_ui: FocusModeUI::new(tx.clone()),
            normal_ui: NormalModeUI::new(tx.clone()),
            popup: None,
            debug_popup: None,
            tx,
        }
    }

    pub fn next_tick(&mut self) {
        // self.focus_ui.next_tick();
        // self.normal_ui.next_tick();
        if let Some(debug_popup) = &mut self.debug_popup {
            debug_popup.next_tick();
            if debug_popup.is_expired() {
                self.debug_popup = None;
            }
        }
    }

    pub fn reset_areas(&mut self) {
        self.focus_ui.reset_areas();
        // self.normal_ui.reset_areas();
    }

    pub fn debug(&mut self, msg: String) {
        self.debug_popup = Some(DebugPopup::new(
            Text::from(msg),
            20, // 2 seconds
            self.tx.clone(),
        ));
    }

    pub fn confirm(&mut self, pending_action: AppAction) {
        let popup = ConfirmationPopup::new(
            Paragraph::new(Text::from("Are you sure?")).centered(),
            pending_action,
            self.tx.clone(),
        );
        self.popup = Some(Box::new(popup));
    }

    // pub fn start_new_task(&mut self) {
    //     let popup = NewTaskPopup::new(self.tx.clone());
    //     self.popup = Some(Box::new(popup));
    // }

    pub fn close_popup(&mut self) {
        if self.debug_popup.is_some() {
            self.debug_popup = None;
            return;
        }
        if self.popup.is_some() {
            self.popup = None;
        }
    }

    pub fn update_tasks(&mut self, tasks: Arc<Vec<Arc<Task>>>) {
        self.focus_ui.update_tasks(Arc::clone(&tasks));
        self.normal_ui.update_tasks(tasks);
    }

    // pub fn apply_view_filter(&mut self, view: &crate::ui::views::View) {
    //     match self.mode {
    //         AppUIMode::Focus => self.focus_ui.filter_by_view(view),
    //         AppUIMode::Normal => {
    //             // The normal mode will handle view filtering internally
    //             // when the view list selection changes
    //         }
    //     }
    // }

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
        if let Some(debug_popup) = &mut self.debug_popup {
            debug_popup.handle_key_event(key_event);
            return;
        }
        if let Some(popup) = &mut self.popup {
            popup.handle_key_event(key_event);
            return;
        }
        match key_event.code {
            KeyCode::F(1) if self.allow_quit() => {
                let _ = self
                    .tx
                    .send(AppAction::Debug(format!("{:?}", self.tx).into()));
            }
            KeyCode::Char('f') if self.mode == AppUIMode::Normal && self.normal_ui.allow_quit() => {
                let view = self
                    .normal_ui
                    .get_current_view()
                    .cloned()
                    .unwrap_or(View::Inbox);
                self.mode = AppUIMode::Focus;
                self.focus_ui.set_view(view);
            }
            KeyCode::Esc if self.mode == AppUIMode::Focus && self.allow_quit() => {
                self.mode = AppUIMode::Normal
            }
            // KeyCode::F(2) => self.mode = AppUIMode::Normal,
            // KeyCode::Char('n') if self.allow_quit() => self.start_new_task(),
            _ => match self.mode {
                AppUIMode::Focus => self.focus_ui.handle_key_event(key_event),
                AppUIMode::Normal => self.normal_ui.handle_key_event(key_event),
            },
        }
    }

    pub fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        if let Some(debug_popup) = &mut self.debug_popup {
            debug_popup.handle_mouse_event(mouse_event);
            return;
        }
        if let Some(popup) = &mut self.popup {
            popup.handle_mouse_event(mouse_event);
            return;
        }
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
        if let Some(popup) = &mut self.popup {
            popup.draw(f, area, last_frame);
        }
        if let Some(debug_popup) = &mut self.debug_popup {
            debug_popup.draw(f, area, last_frame);
        }
    }
}
