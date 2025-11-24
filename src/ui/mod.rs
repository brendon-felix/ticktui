use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{Frame, layout::Rect, text::Text, widgets::Paragraph};

mod animate;
mod batchmode;
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
mod viewselector;

use focusmode::FocusModeUI;
use normalmode::NormalModeUI;
use std::{sync::Arc, time::Instant};
use ticks::tasks::Task;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::AppAction,
    ui::{
        batchmode::BatchModeUI,
        focusmode::FocusModeAction,
        normalmode::NormalModeAction,
        popup::{Popup, confirm::ConfirmationPopup, debug::DebugPopup, newtask::NewTaskPopup},
        viewselector::View,
    },
};

#[derive(Debug, Clone)]
pub enum UIAction {
    EnterFocusMode(View),
    EnterBatchMode(View),
    ExitToNormalMode,
    NormalMode(NormalModeAction),
    FocusMode(FocusModeAction),
    Confirm(Box<AppAction>),
    NewTask,
    ClosePopup,
    DebugMsg(String, u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AppUIMode {
    Normal,
    Focus,
    Batch,
}

pub struct AppUI {
    mode: AppUIMode,
    normal_ui: NormalModeUI,
    focus_ui: FocusModeUI,
    batch_ui: BatchModeUI,
    popup: Option<Box<dyn Popup>>,
    debug_popup: Option<DebugPopup>,
    tx: UnboundedSender<AppAction>,
}

impl AppUI {
    pub fn new(tx: UnboundedSender<AppAction>) -> Self {
        Self {
            mode: AppUIMode::Normal,
            normal_ui: NormalModeUI::new(tx.clone()),
            focus_ui: FocusModeUI::new(),
            batch_ui: BatchModeUI::new(),
            popup: None,
            debug_popup: None,
            tx,
        }
    }

    pub fn execute_action(&mut self, action: UIAction, tx: &UnboundedSender<AppAction>) {
        match action {
            UIAction::EnterFocusMode(view) => {
                self.mode = AppUIMode::Focus;
                self.focus_ui.set_view(view);
            }
            UIAction::ExitToNormalMode => {
                self.mode = AppUIMode::Normal;
            }
            UIAction::EnterBatchMode(view) => {
                self.mode = AppUIMode::Batch;
            }
            UIAction::NormalMode(normal_action) => {
                self.normal_ui.execute_action(normal_action, tx);
            }
            UIAction::FocusMode(focus_action) => {
                self.focus_ui.execute_action(focus_action);
            }
            UIAction::Confirm(pending_action) => {
                self.confirm(*pending_action);
            }
            UIAction::NewTask => {
                self.new_task();
            }
            UIAction::ClosePopup => {
                self.close_popup();
            }
            UIAction::DebugMsg(msg, n_ticks) => {
                self.debug(msg, n_ticks);
            }
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

    pub fn debug(&mut self, msg: String, n_ticks: u16) {
        self.debug_popup = Some(DebugPopup::new(
            Text::from(msg),
            n_ticks, // 2 seconds
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

    pub fn new_task(&mut self) {
        let view = match self.mode {
            AppUIMode::Normal => self.normal_ui.get_current_view(),
            AppUIMode::Focus => self.focus_ui.get_view().cloned(),
            AppUIMode::Batch => return,
        };
        let view = view.unwrap_or(View::Today);
        let popup = NewTaskPopup::new(view, self.tx.clone());
        self.popup = Some(Box::new(popup));
    }

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

    pub fn allow_key_cmd(&self) -> bool {
        match &self.popup {
            Some(p) => p.allow_key_cmd(),
            None => match self.mode {
                AppUIMode::Normal => self.normal_ui.allow_key_cmd(),
                AppUIMode::Focus => self.focus_ui.allow_key_cmd(),
                AppUIMode::Batch => self.batch_ui.allow_key_cmd(),
            },
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent, tx: &UnboundedSender<AppAction>) {
        // 'q' and 'ctrl+c' are handled by app.rs
        match key_event.code {
            KeyCode::Char('n') if self.allow_key_cmd() => {
                let _ = tx.send(AppAction::UIAction(UIAction::NewTask));
            }
            _ => {
                if let Some(debug_popup) = &mut self.debug_popup {
                    debug_popup.handle_key_event(key_event);
                    return;
                }
                if let Some(popup) = &mut self.popup {
                    popup.handle_key_event(key_event);
                    return;
                }
                match self.mode {
                    AppUIMode::Normal => self.normal_ui.handle_key_event(key_event, tx),
                    AppUIMode::Focus => self.focus_ui.handle_key_event(key_event, tx),
                    AppUIMode::Batch => self.batch_ui.handle_key_event(key_event, tx),
                }
            }
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
            AppUIMode::Normal => self.normal_ui.handle_mouse_event(mouse_event),
            AppUIMode::Focus => self.focus_ui.handle_mouse_event(mouse_event),
            AppUIMode::Batch => self.batch_ui.handle_mouse_event(mouse_event),
        }
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect, last_frame: Instant) {
        match self.mode {
            AppUIMode::Normal => self.normal_ui.draw(f, area, last_frame),
            AppUIMode::Focus => self.focus_ui.draw(f, area, last_frame),
            AppUIMode::Batch => self.batch_ui.draw(f, area, last_frame),
        }
        if let Some(popup) = &mut self.popup {
            popup.draw(f, area, last_frame);
        }
        if let Some(debug_popup) = &mut self.debug_popup {
            debug_popup.draw(f, area, last_frame);
        }
    }
}

pub fn debug_msg(msg: &str, n_ticks: u16, tx: &UnboundedSender<AppAction>) {
    let _ = tx.send(AppAction::UIAction(UIAction::DebugMsg(
        msg.to_string(),
        n_ticks,
    )));
}
