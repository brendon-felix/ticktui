use chrono::Local;
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
use ticks::tasks::{Task, TaskID, TaskPriority};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::AppAction,
    tasks::{TaskAction, TaskData, is_overdue},
    ui::{
        animate::{Animation, AnimationDirection, AnimationType},
        focuslist::{
            FocusList, FocusListItem,
            state::{FocusListPosition, FocusListState},
        },
        utils,
        views::View,
    },
};

#[derive(Debug, Clone)]
pub enum FocusModeAction {}

pub struct FocusModeUI {
    // test_content: String,
    all_tasks: Arc<Vec<Arc<Task>>>,
    shown_tasks: Vec<TaskID>,
    current_view: Option<View>,
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
            all_tasks: Arc::new(Vec::new()),
            shown_tasks: Vec::new(),
            current_view: None,
            list: FocusList::new(Vec::<FocusListItem>::new()),
            list_state: FocusListState::default(),
            effects: EffectManager::default(),
            pending_completion: None,
            tx,
        }
    }

    pub fn reset_areas(&mut self) {
        self.list_state.reset_areas();
    }

    pub fn set_all_tasks(&mut self, tasks: Arc<Vec<Arc<Task>>>) {
        self.all_tasks = tasks;
    }

    pub fn set_view(&mut self, view: View) {
        self.shown_tasks = view.get_filtered_task_ids(Local::now(), self.all_tasks.as_ref());
        if self.shown_tasks.is_empty() {
            self.list.focus(None);
        } else {
            self.list.focus(Some(0));
        }
        self.current_view = Some(view);
    }

    pub fn update_tasks(&mut self, tasks: Arc<Vec<Arc<Task>>>) {
        self.set_all_tasks(tasks);
        // Apply the current view filter
        if let Some(current_view) = &self.current_view {
            self.shown_tasks =
                current_view.get_filtered_task_ids(Local::now(), self.all_tasks.as_ref());
        } else {
            self.shown_tasks = self
                .all_tasks
                .iter()
                .map(|task| task.get_id())
                .cloned()
                .collect();
        }
        if self.shown_tasks.is_empty() {
            self.list.focus(None);
        } else if let Some(selected) = self.list.focused_index() {
            if selected >= self.shown_tasks.len() {
                self.list.focus(Some(self.shown_tasks.len() - 1));
            }
        } else {
            self.list.focus(Some(0));
        }
        // self.tasks_loaded = true;
    }

    // pub fn update_tasks(&mut self, tasks: Arc<Vec<Arc<Task>>>) {
    //     let len_before = self.shown_tasks.len();
    //     self.set_all_tasks(tasks);
    //     self.filter_tasks(|now, task| is_due_today(now, task) | is_overdue(now, task));
    //     if len_before != self.filtered_indices.len() {
    //         let duration = Duration::from_millis(1000);
    //         [
    //             FocusListPosition::Prev,
    //             FocusListPosition::Focused,
    //             FocusListPosition::Next,
    //         ]
    //         .iter()
    //         .for_each(|pos| {
    //             if let Some(area_ref) = self.list_state.get_area_ref(*pos) {
    //                 let fx = fx::dynamic_area(
    //                     area_ref,
    //                     fx::fade_from_fg(
    //                         Color::Rgb(25, 25, 25),
    //                         EffectTimer::new(duration.into(), tachyonfx::Interpolation::SineOut),
    //                     ),
    //                 );
    //                 self.effects.add_effect(fx);
    //             }
    //         });
    //     }
    //     // self.task_list.tasks_loaded = true;
    // }

    pub fn schedule_completion(&mut self, delay_ms: u64) {
        if let Some(idx) = self.list.focused_index() {
            let removal_time = Instant::now() + Duration::from_millis(delay_ms);
            self.pending_completion = Some((idx, removal_time));
        }
    }

    pub fn process_pending_completion(&mut self) {
        if let Some((shown_idx, removal_time)) = self.pending_completion {
            if Instant::now() >= removal_time {
                if shown_idx < self.shown_tasks.len() {
                    let task_id = &self.shown_tasks[shown_idx];
                    if let Some(task) = self.all_tasks.iter().find(|t| t.get_id() == task_id) {
                        let data = TaskData::from_task(&task);
                        self.tx
                            .send(AppAction::MultiAction(vec![
                                AppAction::TaskAction(TaskAction::Complete, data),
                                AppAction::RefreshData,
                            ]))
                            .unwrap_or(());
                    }
                    self.shown_tasks.remove(shown_idx);
                    if self.shown_tasks.is_empty() {
                        self.list.focus(None);
                    } else if shown_idx >= self.shown_tasks.len() {
                        self.list.focus(Some(self.shown_tasks.len() - 1));
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
            KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('e') if self.list.len() > 0 => {
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
                    if i + 1 < self.shown_tasks.len() {
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

        // let main_chunks = Layout::new(
        //     Direction::Vertical,
        //     [Constraint::Length(3), Constraint::Fill(1)],
        // )
        // .split(area);

        // if let Some(view) = self.current_view.as_ref() {
        //     let header_area = Layout::new(
        //         Direction::Horizontal,
        //         [
        //             Constraint::Fill(1),
        //             Constraint::Length(20),
        //             Constraint::Fill(1),
        //         ],
        //     )
        //     .split(main_chunks[0])[1];

        //     let p = Paragraph::new(format!("\n{}\n", view.get_name()))
        //         .style(Style::default().fg(Color::LightYellow))
        //         // .block(
        //         //     Block::new()
        //         //         .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        //         //         .border_type(BorderType::Rounded),
        //         // )
        //         .alignment(ratatui::layout::Alignment::Center);
        //     f.render_widget(p, header_area);
        // }

        // let items: Vec<FocusListItem> = self
        //     .filtered_indices
        //     .iter()
        //     .filter_map(|&idx| self.all_tasks.get(idx))
        //     .map(|task| create_list_item(task))
        //     .collect();
        let items: Vec<FocusListItem> = self
            .shown_tasks
            .iter()
            .filter_map(|task_id| self.all_tasks.iter().find(|t| t.get_id() == task_id))
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

    let line1 = Line::from("");
    let line2 = Line::from(task.title.clone());
    // let line3 = if let Some(date_str) = format_date(&task.due_date, task.is_all_day, is_today) {
    let datetime_str = utils::format_datetime(task.due_date, task.is_all_day);
    let line3 = {
        let mut line = Line::from(datetime_str);
        if is_overdue(now, task) {
            line = line.style(Style::default().fg(Color::Red).dim());
        } else {
            line = line.style(Style::default().dim());
        }
        line
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
