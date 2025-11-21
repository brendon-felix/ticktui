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
use tachyonfx::{EffectManager, EffectTimer, Interpolation, RefRect, fx};
use ticks::tasks::{Task, TaskID, TaskPriority};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::AppAction,
    tasks::{TaskAction, TaskData, is_overdue},
    term::TICK_PERIOD_MS,
    ui::{
        UIAction,
        animate::{Animation, AnimationDirection, AnimationType},
        focuslist::{
            FocusList, FocusListItem,
            state::{FocusListPosition, FocusListState},
        },
        utils,
        viewselector::View,
    },
};

const N_TICKS: u64 = 3;
const EXPLODE_MS: u64 = TICK_PERIOD_MS * N_TICKS;
const MOTION_DURATION_MS: u64 = TICK_PERIOD_MS * 2;

#[derive(Debug, Clone)]
pub enum FocusModeAction {
    AnimateScrollUp,
    AnimateScrollDown,
    AnimateShiftUp,
    AnimateShiftDown,
    // AnimateCompletion(bool), // is_next_available
    AnimateCompletion,
    RemoveFocusedItem,
}

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
    // tx: UnboundedSender<AppAction>,
}

impl FocusModeUI {
    pub fn new() -> Self {
        Self {
            all_tasks: Arc::new(Vec::new()),
            shown_tasks: Vec::new(),
            current_view: None,
            list: FocusList::new(Vec::<FocusListItem>::new()),
            list_state: FocusListState::default(),
            effects: EffectManager::default(),
            // tx,
        }
    }

    pub fn execute_action(&mut self, action: FocusModeAction) {
        match action {
            FocusModeAction::AnimateScrollUp => self.animate_scroll_up(),
            FocusModeAction::AnimateScrollDown => self.animate_scroll_down(),
            FocusModeAction::AnimateShiftUp => self.animate_shift_up(),
            FocusModeAction::AnimateShiftDown => self.animate_shift_down(),
            FocusModeAction::AnimateCompletion => self.animate_completion(),
            FocusModeAction::RemoveFocusedItem => self.remove_focused_item(),
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

    fn get_focused_task(&self) -> Option<(usize, Arc<Task>)> {
        if let Some(idx) = self.list.focused_index() {
            if let Some(task_id) = self.shown_tasks.get(idx) {
                if let Some(task) = self.all_tasks.iter().find(|t| t.get_id() == task_id) {
                    return Some((idx, Arc::clone(&task)));
                }
            }
        }
        None
    }

    fn remove_focused_item(&mut self) {
        if let Some((idx, _task)) = self.get_focused_task() {
            self.shown_tasks.remove(idx);
            if self.shown_tasks.is_empty() {
                self.list.focus(None);
            } else if idx >= self.shown_tasks.len() {
                self.list.focus(Some(self.shown_tasks.len() - 1));
            }
        }
    }

    pub fn allow_key_cmd(&self) -> bool {
        true
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent, tx: &UnboundedSender<AppAction>) {
        match key_event.code {
            KeyCode::Char('j') | KeyCode::Down if self.list.focus_next() => {
                // self.animate_scroll_down();
                let _ = tx.send(AppAction::UIAction(UIAction::FocusMode(
                    FocusModeAction::AnimateScrollDown,
                )));
            }
            KeyCode::Char('k') | KeyCode::Up if self.list.focus_previous() => {
                let _ = tx.send(AppAction::UIAction(UIAction::FocusMode(
                    FocusModeAction::AnimateScrollUp,
                )));
            }
            KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('e') if self.list.len() > 0 => {
                if let Some((i, task)) = self.get_focused_task() {
                    let _ = tx.send(AppAction::UIAction(UIAction::FocusMode(
                        FocusModeAction::AnimateCompletion,
                    )));
                    let data = TaskData::from_task(&task);
                    let shift_action = if i + 1 < self.shown_tasks.len() {
                        FocusModeAction::AnimateShiftUp
                    } else {
                        FocusModeAction::AnimateShiftDown
                    };
                    let _ = tx.send(AppAction::AfterNTicks(
                        N_TICKS as u32 - 1,
                        Box::new(AppAction::MultiAction(vec![
                            AppAction::TaskAction(TaskAction::Complete, data),
                            AppAction::UIAction(UIAction::FocusMode(
                                FocusModeAction::RemoveFocusedItem,
                            )),
                            AppAction::UIAction(UIAction::FocusMode(shift_action)),
                        ])),
                    ));
                }
            }
            KeyCode::Esc => {
                let _ = tx.send(AppAction::UIAction(UIAction::ExitFocusMode));
            }
            _ => {}
        }
    }

    pub fn handle_mouse_event(&mut self, _mouse_event: MouseEvent) {
        // Handle mouse events specific to Focus Mode here
    }

    fn animate_scroll_down(&mut self) {
        let duration = Duration::from_millis(MOTION_DURATION_MS);

        // animate the focused (center) item coming from next position (bottom)
        let translate_from_and_grow = focus_from_animation(10, -10, duration);
        self.list_state
            .start_animation(translate_from_and_grow, FocusListPosition::Focused);

        // animate the previous (top) item coming from focused position (center)
        let translate_from_and_shrink = focus_from_animation(10, 10, duration);
        self.list_state
            .start_animation(translate_from_and_shrink, FocusListPosition::Prev);

        // animate the next (bottom) item coming from the bottom edge and fade in
        let translate_from = translate_from_animation(10, duration);
        self.list_state
            .start_animation(translate_from, FocusListPosition::Next);
        if let Some(area_ref) = self.list_state.get_area_ref(FocusListPosition::Next) {
            fade_in(&mut self.effects, area_ref, duration);
        }

        // animate the previous (top) item going to top edge and fade out
        let translate_to = translate_to_animation(-10, duration);
        self.list_state
            .start_animation(translate_to, FocusListPosition::PrevPrev);
        if let Some(area_ref) = self.list_state.get_area_ref(FocusListPosition::PrevPrev) {
            fade_out(&mut self.effects, area_ref, duration);
        }
    }

    fn animate_scroll_up(&mut self) {
        let duration = Duration::from_millis(MOTION_DURATION_MS);

        // animate the focused (center) item coming from previous position (top)
        let translate_from_and_grow = focus_from_animation(-10, -10, duration);
        self.list_state
            .start_animation(translate_from_and_grow, FocusListPosition::Focused);

        // animate the next (bottom) item coming from focused position (center)
        let translate_from_and_shrink = focus_from_animation(-10, 10, duration);
        self.list_state
            .start_animation(translate_from_and_shrink, FocusListPosition::Next);

        // animate the previous (top) item coming from top edge and fade in
        let translate_from = translate_from_animation(-10, duration);
        self.list_state
            .start_animation(translate_from, FocusListPosition::Prev);
        if let Some(area_ref) = self.list_state.get_area_ref(FocusListPosition::Prev) {
            fade_in(&mut self.effects, area_ref, duration);
        }

        // animate the next (bottom) item going to bottom edge and fade out
        let translate_to = translate_to_animation(10, duration);
        self.list_state
            .start_animation(translate_to, FocusListPosition::NextNext);
        if let Some(area_ref) = self.list_state.get_area_ref(FocusListPosition::NextNext) {
            fade_out(&mut self.effects, area_ref, duration);
        }
    }

    fn animate_shift_down(&mut self) {
        let duration = Duration::from_millis(MOTION_DURATION_MS);

        // animate the focused (center) item coming from previous position (top)
        let translate_from_and_grow = focus_from_animation(-10, -10, duration);
        self.list_state
            .start_animation(translate_from_and_grow, FocusListPosition::Focused);

        // animate the previous (top) item coming from the top edge and fade in
        let translate_from = translate_from_animation(-10, duration);
        self.list_state
            .start_animation(translate_from, FocusListPosition::Prev);
        if let Some(area_ref) = self.list_state.get_area_ref(FocusListPosition::Prev) {
            fade_in(&mut self.effects, area_ref, duration);
        }
    }

    fn animate_shift_up(&mut self) {
        let duration = Duration::from_millis(MOTION_DURATION_MS);

        // animate the focused (center) item coming from next position (bottom)
        let translate_from_and_grow = focus_from_animation(10, -10, duration);
        self.list_state
            .start_animation(translate_from_and_grow, FocusListPosition::Focused);

        // animate the next (bottom) item coming from the bottom edge and fade in
        let translate_from = translate_from_animation(10, duration);
        self.list_state
            .start_animation(translate_from, FocusListPosition::Next);
        if let Some(area_ref) = self.list_state.get_area_ref(FocusListPosition::Next) {
            fade_in(&mut self.effects, area_ref, duration);
        }
    }

    fn animate_completion(&mut self) {
        // animate explosion of focused item
        let duration = Duration::from_millis(EXPLODE_MS);
        if let Some(area_ref) = self.list_state.get_area_ref(FocusListPosition::Focused) {
            explode(&mut self.effects, area_ref, duration);
        }
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect, last_frame: Instant) {
        Clear.render(f.area(), f.buffer_mut());
        Block::default()
            .style(Style::default().bg(Color::Rgb(25, 25, 25)))
            .render(f.area(), f.buffer_mut());

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

fn translate_from_animation(translate_y: i32, duration: Duration) -> Animation {
    let anim = AnimationType::TranslateFrom {
        x: 0,
        y: translate_y,
    };
    Animation::new(anim, duration)
}

fn translate_to_animation(translate_y: i32, duration: Duration) -> Animation {
    let anim = AnimationType::TranslateTo {
        x: 0,
        y: translate_y,
    };
    Animation::new(anim, duration)
}

fn focus_from_animation(translate_y: i32, resize_horizontal: i32, duration: Duration) -> Animation {
    let anim = vec![
        AnimationType::TranslateFrom {
            x: 0,
            y: translate_y,
        },
        AnimationType::ResizeFrom {
            dir: AnimationDirection::Horizontal,
            amount: resize_horizontal,
        },
    ];
    Animation::new(AnimationType::Composite(anim), duration)
}

fn fade_in(effects: &mut EffectManager<()>, area_ref: RefRect, duration: Duration) {
    effects.add_effect(fx::dynamic_area(
        area_ref,
        fx::fade_from_fg(
            Color::Rgb(25, 25, 25),
            EffectTimer::new(duration.into(), Interpolation::SineOut),
        ),
    ));
}

fn fade_out(effects: &mut EffectManager<()>, area_ref: RefRect, duration: Duration) {
    effects.add_effect(fx::dynamic_area(
        area_ref,
        fx::fade_to_fg(
            Color::Rgb(25, 25, 25),
            EffectTimer::new(duration.into(), Interpolation::SineOut),
        ),
    ));
}

fn explode(effects: &mut EffectManager<()>, area_ref: RefRect, duration: Duration) {
    let timer = EffectTimer::new(duration.into(), Interpolation::SineOut);
    let c = Color::Rgb(25, 25, 25);
    let fx = fx::dynamic_area(
        area_ref,
        fx::parallel(&[fx::explode(2.0, 2.0, timer), fx::paint_bg(c, timer)]),
    );
    effects.add_effect(fx);
}

// fn dissolve(effects: &mut EffectManager<()>, area_ref: RefRect, duration: Duration) {
//     let timer = EffectTimer::new(duration.into(), Interpolation::SineOut);
//     let fx = fx::dynamic_area(area_ref, fx::dissolve(timer));
//     effects.add_effect(fx);
// }
