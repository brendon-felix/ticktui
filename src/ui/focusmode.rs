use chrono::{DateTime, Local};
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget},
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tachyonfx::{EffectManager, EffectTimer, fx};
use ticks::tasks::{Task, TaskPriority};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::AppAction,
    tasks::{TaskAction, is_due_today, is_overdue},
    ui::{
        animate::{Animation, AnimationDirection, AnimationType},
        focuslist::{
            FocusList, FocusListItem,
            state::{FocusListPosition, FocusListState},
        },
        utils::format_date,
    },
};

pub struct FocusModeUI {
    // test_content: String,
    tasks: Vec<Arc<Task>>,
    list: FocusList<'static>,
    list_state: FocusListState,
    // prev_buf: Buffer,
    // focus_buf: Buffer,
    // next_buf: Buffer,
    effects: EffectManager<()>,
    pending_completion: Option<(usize, Instant)>, // (task_index, removal_time)
    tx: UnboundedSender<AppAction>,
}

impl FocusModeUI {
    pub fn new(tx: UnboundedSender<AppAction>) -> Self {
        Self {
            tasks: Vec::new(),
            list: FocusList::default(),
            list_state: FocusListState::default(),
            effects: EffectManager::default(),
            pending_completion: None,
            tx,
        }
    }

    // pub fn with_tasks(mut self, tasks: Vec<Arc<Task>>) -> Self {
    //     self.tasks = tasks;
    //     self
    // }

    pub fn set_tasks(&mut self, tasks: Vec<Arc<Task>>) {
        self.tasks = tasks;
        if self.list.focused_index().is_none() && !self.tasks.is_empty() {
            self.list.focus(Some(0));
        } else if self.tasks.is_empty() {
            self.list.focus(None);
        }
    }

    pub fn filter_tasks<F>(&mut self, filter_fn: F)
    where
        F: Fn(DateTime<Local>, &Task) -> bool,
    {
        let now = Local::now();
        self.tasks.retain(|task| filter_fn(now, task));
        if self.tasks.is_empty() {
            self.list.focus(None);
        } else if let Some(selected) = self.list.focused_index() {
            if selected >= self.tasks.len() {
                self.list.focus(Some(self.tasks.len() - 1));
            }
        }
    }

    pub fn update_tasks(&mut self, tasks: Vec<Arc<Task>>) {
        let len_before = self.tasks.len();
        self.set_tasks(tasks);
        self.filter_tasks(|now, task| is_due_today(now, task) | is_overdue(now, task));
        if len_before != self.tasks.len() {
            let duration = Duration::from_millis(1000);
            [
                FocusListPosition::Prev,
                FocusListPosition::Focused,
                FocusListPosition::Next,
            ]
            .iter()
            .for_each(|pos| {
                if let Some(area_ref) = self.list_state.get_area_ref(*pos) {
                    let fx = fx::dynamic_area(
                        area_ref,
                        fx::fade_from_fg(
                            Color::Rgb(25, 25, 25),
                            EffectTimer::new(duration.into(), tachyonfx::Interpolation::SineOut),
                        ),
                    );
                    self.effects.add_effect(fx);
                }
            });
        }
        // self.task_list.tasks_loaded = true;
    }

    pub fn schedule_completion(&mut self, delay_ms: u64) {
        if let Some(idx) = self.list.focused_index() {
            let removal_time = Instant::now() + Duration::from_millis(delay_ms);
            self.pending_completion = Some((idx, removal_time));
        }
    }

    pub fn process_pending_completion(&mut self) {
        if let Some((idx, removal_time)) = self.pending_completion {
            if Instant::now() >= removal_time {
                if idx < self.tasks.len() {
                    let task = self.tasks.remove(idx);
                    let project_id = task.project_id.clone();
                    let task_id = task.get_id().clone();
                    self.tx
                        .send(AppAction::TaskAction(
                            project_id,
                            task_id,
                            TaskAction::Complete,
                        ))
                        .unwrap_or(());
                    if self.tasks.is_empty() {
                        self.list.focus(None);
                    } else if idx >= self.tasks.len() {
                        self.list.focus(Some(self.tasks.len() - 1));
                    }
                }
                self.pending_completion = None;
            }
        }
    }

    pub fn allow_quit(&self) -> bool {
        true
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        let idx = self.list.focused_index();
        match key_event.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.list.focus_next();
                if idx != self.list.focused_index() {
                    let translate = AnimationType::TranslateFrom { x: 0, y: 10 };
                    let translate_shrink = vec![
                        AnimationType::TranslateFrom { x: 0, y: 10 },
                        AnimationType::ResizeFrom {
                            dir: AnimationDirection::Horizontal,
                            amount: 10,
                        },
                    ];
                    let translate_grow = vec![
                        AnimationType::TranslateFrom { x: 0, y: 10 },
                        AnimationType::ResizeFrom {
                            dir: AnimationDirection::Horizontal,
                            amount: -10,
                        },
                    ];
                    let translate_shrink = AnimationType::Composite(translate_shrink);
                    let translate_grow = AnimationType::Composite(translate_grow);
                    let duration = Duration::from_millis(200);
                    self.list_state.start_animation(
                        Animation::new(translate_grow, duration),
                        FocusListPosition::Focused,
                    );
                    self.list_state.start_animation(
                        Animation::new(translate_shrink, duration),
                        FocusListPosition::Prev,
                    );
                    self.list_state.start_animation(
                        Animation::new(translate.clone(), duration),
                        FocusListPosition::Next,
                    );
                    if let Some(area_ref) = self.list_state.get_area_ref(FocusListPosition::Next) {
                        let fx = fx::dynamic_area(
                            area_ref,
                            fx::fade_from_fg(
                                Color::Rgb(25, 25, 25),
                                EffectTimer::new(
                                    duration.into(),
                                    tachyonfx::Interpolation::SineOut,
                                ),
                            ),
                        );
                        self.effects.add_effect(fx);
                    }
                    let translate = AnimationType::TranslateTo { x: 0, y: -10 };
                    self.list_state.start_animation(
                        Animation::new(translate.clone(), duration),
                        FocusListPosition::PrevPrev,
                    );
                    if let Some(area_ref) =
                        self.list_state.get_area_ref(FocusListPosition::PrevPrev)
                    {
                        let fx = fx::dynamic_area(
                            area_ref,
                            fx::fade_to_fg(
                                Color::Rgb(25, 25, 25),
                                EffectTimer::new(
                                    duration.into(),
                                    tachyonfx::Interpolation::SineOut,
                                ),
                            ),
                        );
                        self.effects.add_effect(fx);
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.list.focus_previous();
                if idx != self.list.focused_index() {
                    let translate = AnimationType::TranslateFrom { x: 0, y: -10 };
                    let translate_shrink = vec![
                        AnimationType::TranslateFrom { x: 0, y: -10 },
                        AnimationType::ResizeFrom {
                            dir: AnimationDirection::Horizontal,
                            amount: 10,
                        },
                    ];
                    let translate_grow = vec![
                        AnimationType::TranslateFrom { x: 0, y: -10 },
                        AnimationType::ResizeFrom {
                            dir: AnimationDirection::Horizontal,
                            amount: -10,
                        },
                    ];
                    let translate_shrink = AnimationType::Composite(translate_shrink);
                    let translate_grow = AnimationType::Composite(translate_grow);
                    let duration = Duration::from_millis(200);
                    self.list_state.start_animation(
                        Animation::new(translate_grow, duration),
                        FocusListPosition::Focused,
                    );
                    self.list_state.start_animation(
                        Animation::new(translate, duration),
                        FocusListPosition::Prev,
                    );
                    self.list_state.start_animation(
                        Animation::new(translate_shrink, duration),
                        FocusListPosition::Next,
                    );
                    if let Some(area_ref) = self.list_state.get_area_ref(FocusListPosition::Prev) {
                        let fx = fx::dynamic_area(
                            area_ref,
                            fx::fade_from_fg(
                                Color::Rgb(25, 25, 25),
                                EffectTimer::new(
                                    duration.into(),
                                    tachyonfx::Interpolation::SineOut,
                                ),
                            ),
                        );
                        self.effects.add_effect(fx);
                    }
                    let translate = AnimationType::TranslateTo { x: 0, y: 10 };
                    self.list_state.start_animation(
                        Animation::new(translate.clone(), duration),
                        FocusListPosition::NextNext,
                    );
                    if let Some(area_ref) =
                        self.list_state.get_area_ref(FocusListPosition::NextNext)
                    {
                        let fx = fx::dynamic_area(
                            area_ref,
                            fx::fade_to_fg(
                                Color::Rgb(25, 25, 25),
                                EffectTimer::new(
                                    duration.into(),
                                    tachyonfx::Interpolation::SineOut,
                                ),
                            ),
                        );
                        self.effects.add_effect(fx);
                    }
                }
            }
            KeyCode::Enter if self.list.len() > 0 => {
                self.schedule_completion(300);
                if let Some(area_ref) = self.list_state.get_area_ref(FocusListPosition::Focused) {
                    let duration = Duration::from_millis(300);
                    let timer =
                        EffectTimer::new(duration.into(), tachyonfx::Interpolation::SineOut);
                    let c = Color::Rgb(25, 25, 25);
                    let fx = fx::dynamic_area(
                        area_ref,
                        fx::parallel(&[fx::explode(2.0, 2.0, timer), fx::paint_bg(c, timer)]),
                    );
                    // let fx = fx::dynamic_area(area_ref, fx::fade_to_fg(c, timer));
                    // let fx = fx::dynamic_area(area_ref, fx::dissolve(timer)); // use this for deleting tasks instead
                    self.effects.add_effect(fx);
                }
                if let Some(i) = self.list.focused_index() {
                    if i + 1 < self.tasks.len() {
                        let translate = AnimationType::TranslateFrom { x: 0, y: 10 };
                        let translate_grow = vec![
                            AnimationType::TranslateFrom { x: 0, y: 10 },
                            AnimationType::ResizeFrom {
                                dir: AnimationDirection::Horizontal,
                                amount: -10,
                            },
                        ];
                        let translate_grow = AnimationType::Composite(translate_grow);
                        let duration = Duration::from_millis(200);
                        let delay = Duration::from_millis(300);
                        self.list_state.start_animation(
                            Animation::new(
                                AnimationType::Delay(delay, Box::new(translate_grow)),
                                duration,
                            ),
                            FocusListPosition::Focused,
                        );
                        self.list_state.start_animation(
                            Animation::new(
                                AnimationType::Delay(delay, Box::new(translate)),
                                duration,
                            ),
                            FocusListPosition::Next,
                        );
                        if let Some(area_ref) =
                            self.list_state.get_area_ref(FocusListPosition::Next)
                        {
                            let fx = fx::delay(
                                EffectTimer::new(delay.into(), tachyonfx::Interpolation::Linear),
                                fx::dynamic_area(
                                    area_ref,
                                    fx::fade_from_fg(
                                        Color::Rgb(25, 25, 25),
                                        EffectTimer::new(
                                            duration.into(),
                                            tachyonfx::Interpolation::SineOut,
                                        ),
                                    ),
                                ),
                            );
                            self.effects.add_effect(fx);
                        }
                    } else {
                        let translate = AnimationType::TranslateFrom { x: 0, y: -10 };
                        let translate_grow = vec![
                            AnimationType::TranslateFrom { x: 0, y: -10 },
                            AnimationType::ResizeFrom {
                                dir: AnimationDirection::Horizontal,
                                amount: -10,
                            },
                        ];
                        let translate_grow = AnimationType::Composite(translate_grow);
                        let duration = Duration::from_millis(200);
                        let delay = Duration::from_millis(300);
                        self.list_state.start_animation(
                            Animation::new(
                                AnimationType::Delay(delay, Box::new(translate_grow)),
                                duration,
                            ),
                            FocusListPosition::Focused,
                        );
                        self.list_state.start_animation(
                            Animation::new(
                                AnimationType::Delay(delay, Box::new(translate)),
                                duration,
                            ),
                            FocusListPosition::Prev,
                        );
                        if let Some(area_ref) =
                            self.list_state.get_area_ref(FocusListPosition::Prev)
                        {
                            let fx = fx::delay(
                                EffectTimer::new(delay.into(), tachyonfx::Interpolation::Linear),
                                fx::dynamic_area(
                                    area_ref,
                                    fx::fade_from_fg(
                                        Color::Rgb(25, 25, 25),
                                        EffectTimer::new(
                                            duration.into(),
                                            tachyonfx::Interpolation::SineOut,
                                        ),
                                    ),
                                ),
                            );
                            self.effects.add_effect(fx);
                        }
                    }
                }
                // }
            }
            _ => {}
        }
        if idx != self.list.focused_index() {
            //     let (prev, focus, next) = self.list_state.get_sub_areas();
            //     let c = Color::Rgb(25, 25, 25);
            //     let timer = EffectTimer::from_ms(100, Interpolation::Linear);
            //     if let Some(a) = prev {
            //         let fx = fx::fade_from_fg(c, timer).with_area(a);
            //         self.effects.add_effect(fx);
            //     }
            //     if let Some(a) = focus {
            //         let fx = fx::fade_from_fg(c, timer).with_area(a);
            //         self.effects.add_effect(fx);
            //     }
            //     if let Some(a) = next {
            //         let fx = fx::fade_from_fg(c, timer).with_area(a);
            //         self.effects.add_effect(fx);
            //     }
        }
    }

    pub fn handle_mouse_event(&mut self, _mouse_event: MouseEvent) {
        // Handle mouse events specific to Focus Mode here
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect, last_frame: Instant) {
        self.process_pending_completion();
        Clear.render(f.area(), f.buffer_mut());
        Block::default()
            .style(Style::default().bg(Color::Rgb(25, 25, 25)))
            .render(f.area(), f.buffer_mut());
        let items: Vec<FocusListItem> = self
            .tasks
            .iter()
            .map(|task| create_list_item(task))
            .collect();
        self.list.set_items(items);

        if self.list.len() > 0 {
            self.list_state.update_animations();
            f.render_stateful_widget(&self.list, area, &mut self.list_state);
        } else {
            render_no_tasks(f, area);
        }

        let elapsed = last_frame.elapsed();
        self.effects
            .process_effects(elapsed.into(), f.buffer_mut(), area);
    }
}

fn create_list_item(task: &Arc<Task>) -> FocusListItem<'static> {
    let now = chrono::Local::now();
    let is_today = is_due_today(now, task);

    let line1 = Line::from("");
    let line2 = Line::from(task.title.clone());
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
    let mut item = FocusListItem::new(vec![line1, line2, line3]);
    match task.priority {
        TaskPriority::High => item = item.with_border_color(Color::Red),
        TaskPriority::Medium => item = item.with_border_color(Color::Yellow),
        TaskPriority::Low => item = item.with_border_color(Color::Blue),
        TaskPriority::None => {}
    }
    item
}

fn render_no_tasks(f: &mut Frame, area: Rect) {
    let r = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Fill(1),
                Constraint::Length(5),
                Constraint::Fill(1),
            ]
            .as_ref(),
        )
        .split(area)[1];
    let r = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Fill(1),
                Constraint::Length(30),
                Constraint::Fill(1),
            ]
            .as_ref(),
        )
        .split(r)[1];
    f.render_widget(
        Paragraph::new("\nNo tasks available").centered().block(
            Block::new()
                .borders(Borders::ALL)
                .border_set(BorderType::Rounded.to_border_set()),
        ),
        r,
    );
}
