use chrono::{DateTime, Local};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Paragraph},
};
use std::{sync::Arc, time::Instant};
use tachyonfx::EffectManager;
use ticks::tasks::Task;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::AppAction,
    tasks::{TaskAction, is_due_today, is_overdue},
    ui::{
        multiselect::{MultiSelectList, MultiSelectListItem, MultiSelectListState},
        utils::format_date,
    },
};

// const ITEM_HEIGHT: u16 = 3;

// pub enum TaskListMode {
//     Normal,
//     Visual,
// }

pub struct TaskList {
    tasks: Vec<Arc<Task>>,
    list_state: MultiSelectListState,
    style: Style,
    current_block: Option<Block<'static>>,
    pub tasks_loaded: bool,
    pub task_changed: bool,
    _effects: EffectManager<()>,
    tx: UnboundedSender<AppAction>,
}

#[allow(dead_code)]
impl TaskList {
    pub fn new(tasks: Vec<Arc<Task>>, tx: UnboundedSender<AppAction>) -> Self {
        let list_state = MultiSelectListState::default();
        let current_block = Some(
            Block::default()
                .title("Tasks")
                .border_set(BorderType::Rounded.to_border_set())
                .borders(ratatui::widgets::Borders::ALL),
        );
        let _effects: EffectManager<()> = EffectManager::default();
        Self {
            tasks,
            list_state,
            style: Style::default(),
            current_block,
            tasks_loaded: false,
            task_changed: true,
            _effects,
            tx,
        }
    }

    pub fn activate(&mut self) {
        if self.tasks.is_empty() {
            self.list_state.select(None);
        } else if self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        }
        self.current_block = Some(
            Block::default()
                .title("Tasks")
                .border_set(BorderType::Rounded.to_border_set())
                .borders(ratatui::widgets::Borders::ALL),
        );
        self.style = Style::default();
    }

    pub fn deactivate(&mut self) {
        self.current_block = Some(
            Block::default()
                .title("Tasks")
                .borders(ratatui::widgets::Borders::ALL)
                .border_set(BorderType::Rounded.to_border_set())
                .style(Style::default().add_modifier(Modifier::DIM)),
        );
        self.style = Style::default().add_modifier(Modifier::DIM);
    }

    pub fn filter_tasks<F>(&mut self, filter_fn: F)
    where
        F: Fn(DateTime<Local>, &Task) -> bool,
    {
        let now = Local::now();
        self.tasks.retain(|task| filter_fn(now, task));
        if self.tasks.is_empty() {
            self.list_state.select(None);
        } else if let Some(selected) = self.list_state.selected() {
            if selected >= self.tasks.len() {
                self.list_state.select(Some(self.tasks.len() - 1));
            }
        }
    }

    pub fn with_tasks(mut self, tasks: Vec<Arc<Task>>) -> Self {
        self.tasks = tasks;
        self
    }

    pub fn set_tasks(&mut self, tasks: Vec<Arc<Task>>) {
        self.tasks = tasks;
        if self.tasks.is_empty() {
            self.list_state.select(None);
        }
        if self.list_state.selected().is_none() && !self.tasks.is_empty() {
            self.list_state.select(Some(0));
            self.task_changed = true;
        } else if self.tasks.is_empty() {
            self.list_state.select(None);
        }
    }

    pub fn add_task(&mut self, _task: Arc<Task>) {
        // self.tasks.push(Task::new(task));
    }

    pub fn remove_task(&mut self, index: usize) {
        if index < self.tasks.len() {
            if let Some(task) = self.tasks.get(index) {
                let project_id = task.project_id.clone();
                let task_id = task.get_id().clone();
                let task_action =
                    AppAction::TaskAction(project_id.clone(), task_id.clone(), TaskAction::Delete);
                let confirm_action = AppAction::Confirm(Box::new(AppAction::MultiAction(vec![
                    task_action,
                    AppAction::RefreshTasks,
                ])));
                let _ = self.tx.send(confirm_action);
                if self.tasks.is_empty() {
                    self.list_state.select(None);
                } else if index >= self.tasks.len() {
                    self.list_state.select(Some(self.tasks.len() - 1));
                }
            }
        }
    }

    // pub fn remove_range_inclusive(&mut self, range: (usize, usize)) {
    //     let (start, end) = range;
    //     if start < self.tasks.len() && end < self.tasks.len() && start <= end {
    //         self.tasks.drain(start..=end);
    //     }
    // }

    pub fn remove_selected_tasks(&mut self) {
        if let Some(curr) = self.list_state.selected() {
            if let Some(start) = self.list_state.visual_start {
                let (s, e) = if curr >= start {
                    (start, curr)
                } else {
                    (curr, start)
                };
                // self.remove_range_inclusive((s, e));
                let task_actions = self.tasks[s..=e].iter().map(|task| {
                    let project_id = task.project_id.clone();
                    let task_id = task.get_id().clone();
                    AppAction::TaskAction(project_id, task_id, TaskAction::Delete)
                });
                let confirmation_action = AppAction::Confirm(Box::new(AppAction::MultiAction(
                    task_actions
                        .chain(std::iter::once(AppAction::RefreshTasks))
                        .collect(),
                )));
                let _ = self.tx.send(confirmation_action);
                // self.list_state.select_next();
                self.list_state.select(Some(s));
                self.list_state.end_visual_selection();
            } else {
                self.remove_task(curr);
                self.list_state.select(Some(curr));
            }
        }
    }

    pub fn set_block(&mut self, block: Block<'static>) {
        self.current_block = Some(block);
    }

    // pub fn get_tasks(&self) -> &Vec<TaskItem> {
    //     &self.tasks
    // }

    pub fn get_list_state(&self) -> &MultiSelectListState {
        &self.list_state
    }

    pub fn get_current_task(&self) -> Option<Arc<Task>> {
        if let Some(idx) = self.list_state.selected() {
            self.tasks.get(idx).cloned()
        } else {
            None
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        let idx = self.list_state.selected();
        if self.list_state.is_in_visual_mode() {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => self.list_state.select_next(),
                KeyCode::Char('k') | KeyCode::Up => self.list_state.select_previous(),
                KeyCode::Char('g') => self.list_state.select_first(),
                KeyCode::Char('G') => self.list_state.select_last(),
                KeyCode::Char('d') => self.remove_selected_tasks(),
                KeyCode::Esc => self.list_state.end_visual_selection(),
                _ => {}
            }
            return;
        } else {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => self.list_state.select_next(),
                KeyCode::Char('k') | KeyCode::Up => self.list_state.select_previous(),
                KeyCode::Char('g') => self.list_state.select_first(),
                KeyCode::Char('G') => self.list_state.select_last(),
                KeyCode::Char('v') | KeyCode::Char('V') => self.list_state.start_visual_selection(),
                KeyCode::Char('d') => self.remove_selected_tasks(),
                _ => {}
            }
        }
        self.task_changed = idx != self.list_state.selected();
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect, _last_frame: Instant) {
        if self.tasks.len() == 0 {
            let msg = if !self.tasks_loaded {
                "Loading Tasks..."
            } else {
                "No Tasks Available"
            };
            let mut p = Paragraph::new(msg)
                .style(self.style)
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title("No Tasks")
                        .border_set(BorderType::Rounded.to_border_set())
                        .borders(ratatui::widgets::Borders::ALL),
                );
            if let Some(block) = self.current_block.clone() {
                p = p.block(block);
            }
            f.render_widget(p, area);
            return;
        }

        let items: Vec<MultiSelectListItem> = self
            .tasks
            .iter()
            .map(|task| create_list_item(task))
            .collect();
        let mut task_list = MultiSelectList::new(items)
            .with_style(self.style)
            // .with_highlight_symbol(" ● ")
            .with_highlight_symbol(" ")
            .with_highlight_style(
                Style::new()
                    .bg(Color::Rgb(30, 30, 30))
                    .add_modifier(Modifier::BOLD),
            );

        if let Some(block) = self.current_block.clone() {
            task_list = task_list.with_block(block);
        }
        f.render_stateful_widget(task_list, area, &mut self.list_state);
        // let elapsed = last_frame.elapsed();
        // self.effects
        //     .process_effects(elapsed.into(), f.buffer_mut(), area);
    }
}

fn create_list_item(task: &Arc<Task>) -> MultiSelectListItem<'static> {
    let now = chrono::Local::now();
    let is_today = is_due_today(now, task);
    let line1 = Line::from("");
    let line2 = Line::from(task.title.clone());
    // let line3 = Line::from("");
    let line3 = if let Some(date_str) = format_date(&task.due_date, task.is_all_day, is_today) {
        let mut line = Line::from(date_str);
        if is_overdue(now, task) {
            line = line.style(Style::default().fg(Color::Red).dim());
        } else {
            line = line.style(Style::default().dim());
        }
        line
    } else {
        Line::from("")
    };
    MultiSelectListItem::new(vec![line1, line2, line3])
}

// impl AppWidget for TaskList {
//     fn set_widget_style(&mut self, style: WidgetStyle) {
//         match style {
//             WidgetStyle::Active => {
//                 let is_active = true;
//                 let borders = Borders::ALL;
//                 self.set_block(create_block(self.get_title_owned(), is_active, borders));
//                 self.set_style(Style::default());
//             }
//             WidgetStyle::Inactive => {
//                 let is_active = false;
//                 let borders = Borders::ALL;
//                 self.set_block(create_block(self.get_title_owned(), is_active, borders));
//                 self.set_style(Style::default().add_modifier(Modifier::DIM));
//             }
//         }
//     }

//     fn set_last_area_pos(&mut self, area_pos: Position) {
//         self.last_area_pos = Some(area_pos);
//     }

//     fn on_click(&mut self, pos: Position) {
//         if let Some(area_pos) = self.last_area_pos {
//             let local = Position::new(
//                 pos.x.saturating_sub(area_pos.x),
//                 pos.y.saturating_sub(area_pos.y),
//             );
//             let (_x, y) = if let Some(_block) = &self.current_block {
//                 (local.x.saturating_sub(1), local.y.saturating_sub(1))
//             } else {
//                 local.into()
//             };
//             let index = (y / ITEM_HEIGHT) as usize;
//             if index < self.tasks.len() {
//                 self.list_state.select(Some(index));
//             }
//         }
//     }
// }
