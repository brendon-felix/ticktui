use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Clear, Widget},
};
use std::{sync::Arc, time::Instant};
use tachyonfx::{EffectManager, EffectTimer, Interpolation, Motion, fx};
use ticks::tasks::Task;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::AppAction,
    ui::{
        taskeditor::TaskEditor,
        tasklist::TaskList,
        views::{View, ViewList},
    },
};

enum ActivePane {
    ViewList,
    TaskList,
    TaskEditor,
}

pub struct NormalModeUI {
    view_list: ViewList,
    task_list: TaskList,
    task_editor: TaskEditor,
    active_pane: ActivePane,
    effects: EffectManager<()>,
    // tx: UnboundedSender<AppAction>,
}
impl NormalModeUI {
    pub fn new(tx: UnboundedSender<AppAction>) -> Self {
        let mut view_list = ViewList::new();
        view_list.activate();
        let mut task_list = TaskList::new(Arc::new(vec![]), tx.clone());
        task_list.deactivate();
        let mut task_editor = TaskEditor::new();
        task_editor.deactivate();
        let mut effects: EffectManager<()> = EffectManager::default();
        let c = Color::Rgb(25, 25, 25);
        let timer = EffectTimer::from_ms(500, Interpolation::Linear);
        let fx = fx::sweep_in(Motion::UpToDown, 10, 0, c, timer);
        effects.add_effect(fx);
        Self {
            view_list,
            task_list,
            task_editor,
            active_pane: ActivePane::ViewList,
            effects,
            // tx,
        }
    }

    /// Updates the task editor if the currently selected task has changed.
    /// This is called after task filtering or task list navigation.
    fn update_task_editor_if_needed(&mut self) {
        if self.task_list.task_changed {
            if let Some(selected_task) = self.task_list.get_current_task() {
                self.task_editor.load_task(&selected_task);
            } else {
                // Clear task editor when no task is selected (empty list)
                self.task_editor.clear_all_fields();
            }
            self.task_list.task_changed = false;
        }
    }

    pub fn get_current_view(&self) -> Option<&View> {
        self.view_list.get_current_view()
    }

    fn apply_current_view_filter(&mut self) {
        if let Some(current_view) = self.view_list.get_current_view() {
            self.task_list.filter_by_view(current_view);
        }
    }

    pub fn update_tasks(&mut self, tasks: Arc<Vec<Arc<Task>>>) {
        self.task_list.set_all_tasks(tasks);
        // Apply the current view filter
        self.apply_current_view_filter();
        self.task_list.tasks_loaded = true;
        self.update_task_editor_if_needed();
    }

    // /// Sets the view filter programmatically and applies it to the task list.
    // /// This can be used for testing or external view changes.
    // pub fn set_view_filter(&mut self, view: &View) {
    //     // Update the view list selection to match the provided view
    //     if let Some(index) = self.view_list.views.iter().position(|v| v == view) {
    //         self.view_list.set_selection(index);
    //     }
    //     self.task_list.filter_by_view(view);
    //     if let Some(selected_task) = self.task_list.get_current_task() {
    //         self.task_editor.load_task(&selected_task);
    //     }
    // }

    pub fn allow_quit(&self) -> bool {
        match self.active_pane {
            ActivePane::ViewList => true,
            ActivePane::TaskList => true,
            ActivePane::TaskEditor => !self.task_editor.is_in_insert_mode(),
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.active_pane {
            ActivePane::ViewList => {
                match key_event.code {
                    KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                        self.active_pane = ActivePane::TaskList;
                        self.view_list.deactivate();
                        self.task_list.activate();
                        if let Some(selected_task) = self.task_list.get_current_task() {
                            self.task_editor.load_task(&selected_task);
                        }
                    }
                    _ => self.view_list.handle_key_event(key_event),
                }
                if self.view_list.view_changed {
                    self.apply_current_view_filter();
                    self.task_list.clear_selection();
                    self.task_editor.clear_all_fields();
                    self.view_list.view_changed = false;
                }
            }
            ActivePane::TaskList => {
                match key_event.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        self.active_pane = ActivePane::ViewList;
                        // self.task_list.clear_selection();
                        self.task_editor.clear_all_fields();
                        self.task_list.deactivate();
                        self.view_list.activate();
                    }
                    // KeyCode::Right | KeyCode::Char('l') => {
                    //     self.active_pane = ActivePane::TaskEditor;
                    //     self.task_list.deactivate();
                    //     self.task_editor.activate();
                    // }
                    KeyCode::Enter => {
                        if !self.task_list.is_empty() {
                            self.active_pane = ActivePane::TaskEditor;
                            self.task_list.deactivate();
                            self.task_editor.activate();
                        }
                    }
                    _ => self.task_list.handle_key_event(key_event),
                }
                self.update_task_editor_if_needed();
            }
            ActivePane::TaskEditor => match key_event.code {
                // KeyCode::Left | KeyCode::Char('h') => {
                //     if self.task_editor.is_in_insert_mode() {
                //         self.task_editor.handle_key_event(key_event);
                //     } else {
                //         self.active_pane = ActivePane::TaskList;
                //         self.task_editor.deactivate();
                //         self.task_list.activate();
                //     }
                // }
                KeyCode::Esc => {
                    if self.task_editor.is_in_insert_mode() {
                        self.task_editor.handle_key_event(key_event);
                    } else {
                        self.active_pane = ActivePane::TaskList;
                        self.task_editor.deactivate();
                        self.task_list.activate();
                    }
                }
                _ => self.task_editor.handle_key_event(key_event),
            },
        }
    }

    pub fn handle_mouse_event(&mut self, _mouse_event: MouseEvent) {
        // Handle mouse events specific to Normal Mode here
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect, last_frame: Instant) {
        Clear.render(f.area(), f.buffer_mut());
        Block::default()
            .style(Style::default().bg(Color::Rgb(25, 25, 25)))
            .render(f.area(), f.buffer_mut());

        // let main_area = centered_area(area, 40, 140);
        let main_area = area;

        let chunks = Layout::new(
            Direction::Horizontal,
            // vec![Constraint::Percentage(40), Constraint::Percentage(60)],
            [
                Constraint::Length(15),
                Constraint::Fill(1),
                Constraint::Fill(1),
            ],
        )
        .split(main_area);

        let left_chunks = Layout::new(
            Direction::Vertical,
            [Constraint::Length(17), Constraint::Fill(1)],
        )
        .split(chunks[0]);

        self.view_list.draw(f, left_chunks[0], last_frame);
        self.task_list.draw(f, chunks[1], last_frame);
        self.task_editor.draw(f, chunks[2], last_frame);
        let elapsed = last_frame.elapsed();
        self.effects
            .process_effects(elapsed.into(), f.buffer_mut(), main_area);
    }
}
