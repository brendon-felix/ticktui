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
        UIAction,
        editor::EditorMode,
        taskeditor::TaskEditor,
        tasklist::TaskList,
        viewselector::{View, ViewSelector},
    },
};

#[derive(Debug, Clone)]
pub enum NormalModeAction {
    EditTask(Arc<Task>),
    CreateNewTask,
    ExitTaskEditor,
    SwitchView(View),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ActivePane {
    ViewSelector,
    TaskList,
    TaskEditor,
}

pub struct NormalModeUI {
    view_selector: ViewSelector,
    task_list: TaskList,
    task_editor: TaskEditor,
    active_pane: ActivePane,
    effects: EffectManager<()>,
}
impl NormalModeUI {
    pub fn new(tx: UnboundedSender<AppAction>) -> Self {
        let mut view_selector = ViewSelector::new(tx.clone());
        view_selector.activate();
        let mut task_list = TaskList::new(Arc::new(vec![]), tx.clone());
        task_list.deactivate();
        let mut task_editor = TaskEditor::new(tx.clone());
        task_editor.deactivate();
        let mut effects: EffectManager<()> = EffectManager::default();
        let c = Color::Rgb(25, 25, 25);
        let timer = EffectTimer::from_ms(500, Interpolation::Linear);
        let fx = fx::sweep_in(Motion::UpToDown, 10, 0, c, timer);
        effects.add_effect(fx);
        Self {
            view_selector,
            task_list,
            task_editor,
            active_pane: ActivePane::ViewSelector,
            effects,
        }
    }

    pub fn execute_action(&mut self, action: NormalModeAction) {
        match action {
            NormalModeAction::EditTask(task) => {
                self.task_editor.load_task(&task);
                self.active_pane = ActivePane::TaskEditor;
                self.view_selector.deactivate();
                self.task_list.deactivate();
                self.task_editor.activate();
            }
            NormalModeAction::CreateNewTask => {
                self.task_editor.clear_all_fields();
                if let Some(view) = self.view_selector.get_current_view() {
                    match view {
                        View::Today => self.task_editor.set_due_date_content("Today"),
                        View::Tomorrow => self.task_editor.set_due_date_content("Tomorrow"),
                        View::Week => self.task_editor.set_due_date_content("Today"),
                        // View::Inbox => self.task_editor.set_project_content(""),
                        _ => {}
                    }
                }
                self.active_pane = ActivePane::TaskEditor;
                self.view_selector.deactivate();
                self.task_list.deactivate();
                self.task_editor.activate();
                self.task_editor.set_mode(EditorMode::Insert);
            }
            NormalModeAction::ExitTaskEditor => {
                // if let Some(selected_task) = self.task_list.get_current_task() {
                //     self.task_editor.load_task(&selected_task);
                // } else {
                //     self.task_editor.clear_all_fields();
                // }
                self.task_editor.clear_all_fields();
                self.active_pane = ActivePane::TaskList;
                self.task_editor.deactivate();
                self.view_selector.deactivate();
                self.task_list.activate();
            }
            NormalModeAction::SwitchView(view) => {
                self.task_list.filter_by_view(&view);
                self.task_list.clear_selection();
            }
        }
    }

    fn apply_current_view_filter(&mut self) {
        if let Some(current_view) = self.view_selector.get_current_view() {
            self.task_list.filter_by_view(current_view);
        }
    }

    pub fn update_tasks(&mut self, tasks: Arc<Vec<Arc<Task>>>) {
        self.task_list.set_all_tasks(tasks);
        // Apply the current view filter
        self.apply_current_view_filter();
        self.task_list.tasks_loaded = true;
        // self.update_task_editor_if_needed();
    }

    pub fn allow_key_cmd(&self) -> bool {
        match self.active_pane {
            ActivePane::ViewSelector => true,
            ActivePane::TaskList => true,
            ActivePane::TaskEditor => !self.task_editor.is_in_insert_mode(),
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent, tx: &UnboundedSender<AppAction>) {
        match self.active_pane {
            ActivePane::ViewSelector => match key_event.code {
                // enter task list
                KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                    self.active_pane = ActivePane::TaskList;
                    self.view_selector.deactivate();
                    self.task_list.activate();
                }
                // create new task
                KeyCode::Char('n') => {
                    let _ = tx.send(AppAction::UIAction(UIAction::NormalMode(
                        NormalModeAction::CreateNewTask,
                    )));
                }
                KeyCode::Char('f') if self.allow_key_cmd() => {
                    if let Some(view) = self.view_selector.get_current_view() {
                        let _ =
                            tx.send(AppAction::UIAction(UIAction::EnterFocusMode(view.clone())));
                    }
                }
                _ => self.view_selector.handle_key_event(key_event),
            },
            ActivePane::TaskList => match key_event.code {
                // enter view selector
                KeyCode::Left | KeyCode::Char('h') => {
                    self.active_pane = ActivePane::ViewSelector;
                    self.task_list.deactivate();
                    self.view_selector.activate();
                }
                // edit selected task
                KeyCode::Enter => {
                    if !self.task_list.is_empty()
                        && let Some(task) = self.task_list.get_current_task()
                    {
                        let _ = tx.send(AppAction::UIAction(UIAction::NormalMode(
                            NormalModeAction::EditTask(task),
                        )));
                    }
                }
                // create new task
                KeyCode::Char('n') => {
                    let _ = tx.send(AppAction::UIAction(UIAction::NormalMode(
                        NormalModeAction::CreateNewTask,
                    )));
                }
                KeyCode::Char('f') if self.allow_key_cmd() => {
                    if let Some(view) = self.view_selector.get_current_view() {
                        let _ =
                            tx.send(AppAction::UIAction(UIAction::EnterFocusMode(view.clone())));
                    }
                }
                _ => self.task_list.handle_key_event(key_event),
            },
            ActivePane::TaskEditor => match key_event.code {
                // KeyCode::Left | KeyCode::Char('h') => {
                //     if self.task_editor.is_in_insert_mode() {
                //         self.task_editor.handle_key_event(key_event);
                //     } else {
                //         self.active_pane = ActivePane::ViewSelector;
                //         self.task_editor.deactivate();
                //         self.view_selector.activate();
                //     }
                // }
                KeyCode::Esc => {
                    if self.task_editor.is_in_insert_mode() {
                        self.task_editor.handle_key_event(key_event);
                    } else {
                        if self.task_editor.unsaved_changes {
                            if let Some(selected_task) = self.task_list.get_current_task() {
                                self.task_editor.load_task(&selected_task);
                            } else {
                                self.task_editor.clear_all_fields();
                            }
                        }
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
                // Constraint::Fill(1),
            ],
        )
        .split(main_area);

        let left_chunks = Layout::new(
            Direction::Vertical,
            [Constraint::Length(17), Constraint::Fill(1)],
        )
        .split(chunks[0]);

        self.view_selector.draw(f, left_chunks[0], last_frame);
        match self.active_pane {
            ActivePane::ViewSelector | ActivePane::TaskList => {
                self.task_list.draw(f, chunks[1], last_frame);
            }
            ActivePane::TaskEditor => {
                self.task_editor.draw(f, chunks[1], last_frame);
            }
        }
        let elapsed = last_frame.elapsed();
        self.effects
            .process_effects(elapsed.into(), f.buffer_mut(), main_area);
    }
}
