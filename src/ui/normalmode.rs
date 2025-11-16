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
    tasks::{is_due_today, is_overdue},
    ui::{taskeditor::TaskEditor, tasklist::TaskList},
};

enum ActivePane {
    TaskList,
    TaskEditor,
}

pub struct NormalModeUI {
    task_list: TaskList,
    task_editor: TaskEditor,
    active_pane: ActivePane,
    effects: EffectManager<()>,
    _tx: UnboundedSender<AppAction>,
}
impl NormalModeUI {
    pub fn new(_tx: UnboundedSender<AppAction>) -> Self {
        let mut task_list = TaskList::default();
        task_list.activate();
        let mut task_editor = TaskEditor::new();
        task_editor.deactivate();
        let mut effects: EffectManager<()> = EffectManager::default();
        let c = Color::Rgb(25, 25, 25);
        let timer = EffectTimer::from_ms(500, Interpolation::Linear);
        let fx = fx::sweep_in(Motion::UpToDown, 10, 0, c, timer);
        effects.add_effect(fx);
        Self {
            task_list,
            task_editor,
            active_pane: ActivePane::TaskList,
            effects,
            _tx,
        }
    }

    pub fn update_tasks(&mut self, tasks: Vec<Arc<Task>>) {
        self.task_list.set_tasks(tasks);
        self.task_list
            .filter_tasks(|now, task| is_due_today(now, task) | is_overdue(now, task));
        self.task_list.tasks_loaded = true;
        if self.task_list.task_changed {
            if let Some(selected_task) = self.task_list.get_current_task() {
                self.task_editor.load_task(&selected_task);
            }
            self.task_list.task_changed = false;
        }
    }

    pub fn is_in_insert_mode(&self) -> bool {
        match self.active_pane {
            ActivePane::TaskList => false,
            ActivePane::TaskEditor => self.task_editor.is_in_insert_mode(),
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.active_pane {
            ActivePane::TaskList => match key_event.code {
                KeyCode::Enter => {
                    self.active_pane = ActivePane::TaskEditor;
                    self.task_list.deactivate();
                    self.task_editor.activate();
                }
                _ => self.task_list.handle_key_event(key_event),
            },
            ActivePane::TaskEditor => match key_event.code {
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
        if self.task_list.task_changed {
            if let Some(selected_task) = self.task_list.get_current_task() {
                self.task_editor.load_task(&selected_task);
            }
            self.task_list.task_changed = false;
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
        let main_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Max(40),
                Constraint::Fill(1),
            ])
            .split(area)[1];
        let main_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Max(120),
                Constraint::Fill(1),
            ])
            .split(main_area)[1];

        let chunks = Layout::new(
            Direction::Horizontal,
            vec![Constraint::Percentage(50), Constraint::Percentage(50)],
        )
        .split(main_area);
        self.task_list.draw(f, chunks[0], last_frame);
        self.task_editor.draw(f, chunks[1], last_frame);
        let elapsed = last_frame.elapsed();
        self.effects
            .process_effects(elapsed.into(), f.buffer_mut(), main_area);
    }
}
