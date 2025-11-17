mod focused;
pub mod state;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, StatefulWidget, Widget},
};

use state::FocusListState;

use crate::ui::focuslist::{
    focused::{FocusedItem, NextPrevItem},
    state::FocusListPosition,
};

#[derive(Clone)]
pub struct FocusListItem<'a> {
    content: Text<'a>,
    style: Style,
    border_color: Option<Color>,
}

impl<'a> FocusListItem<'a> {
    pub fn new<T>(content: T) -> Self
    where
        T: Into<Text<'a>>,
    {
        Self {
            content: content.into(),
            style: Style::default(),
            border_color: None,
        }
    }

    pub fn with_border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }
}

impl<'a, T> From<T> for FocusListItem<'a>
where
    T: Into<Text<'a>>,
{
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

#[derive(Default)]
pub struct FocusList<'a> {
    focused_index: Option<usize>,
    block: Option<Block<'a>>,
    items: Vec<FocusListItem<'a>>,
    style: Style,
}

#[allow(dead_code)]
impl<'a> FocusList<'a> {
    pub fn new<T>(items: T) -> Self
    where
        T: IntoIterator,
        T::Item: Into<FocusListItem<'a>>,
    {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn with_block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn with_focused_index(mut self, index: usize) -> Self {
        self.focused_index = Some(index);
        self
    }

    pub fn set_items<T>(&mut self, items: T)
    where
        T: IntoIterator,
        T::Item: Into<FocusListItem<'a>>,
    {
        self.items = items.into_iter().map(Into::into).collect();
    }

    pub fn focused_index(&self) -> Option<usize> {
        self.focused_index
    }

    pub fn next_next_index(&self) -> Option<usize> {
        if let Some(current) = self.focused_index {
            let next = current.saturating_add(2);
            if next < self.items.len() {
                Some(next)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn next_index(&self) -> Option<usize> {
        if let Some(current) = self.focused_index {
            let next = current.saturating_add(1);
            if next < self.items.len() {
                Some(next)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn prev_index(&self) -> Option<usize> {
        if let Some(current) = self.focused_index {
            if current > 0 {
                Some(current.saturating_sub(1))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn prev_prev_index(&self) -> Option<usize> {
        if let Some(current) = self.focused_index {
            if current >= 2 {
                Some(current.saturating_sub(2))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn focus(&mut self, index: Option<usize>) {
        if let Some(i) = index
        // && !(self.focused_index() == Some(i))
        {
            if i >= self.items.len() {
                self.focused_index = Some(self.items.len().saturating_sub(1));
            } else {
                self.focused_index = Some(i);
            }
        } else {
            self.focused_index = None;
        }
    }

    pub fn focus_next(&mut self) {
        let next = self.focused_index.map_or(0, |i| i.saturating_add(1));
        self.focus(Some(next));
    }

    pub fn focus_previous(&mut self) {
        let previous = self
            .focused_index
            .map_or(usize::MAX, |i| i.saturating_sub(1));
        self.focus(Some(previous));
    }

    pub fn focus_first(&mut self) {
        self.focus(Some(0));
    }

    pub fn focus_last(&mut self) {
        self.focus(Some(usize::MAX));
    }
}

impl<'a> StatefulWidget for &FocusList<'a> {
    type State = FocusListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(7), // 1: previous item
                Constraint::Max(3),
                Constraint::Length(7), // 3: focused item
                Constraint::Max(3),
                Constraint::Length(7), // 5: next item
                Constraint::Fill(1),
            ])
            .split(area)
            .iter()
            .enumerate()
            .for_each(|(i, rect)| match i {
                1 => {
                    let area = if let Some(area) = state.get_area(FocusListPosition::PrevPrev) {
                        area
                    } else {
                        let area = constrained_centered_area(*rect, 50);
                        state.set_area(area, FocusListPosition::PrevPrev);
                        area
                    };
                    if let Some(prev_prev) = self.prev_prev_index() {
                        if let Some(anim_area) = &state.prev_prev_area {
                            if !anim_area.is_completed() {
                                let item: FocusListItem<'_> = self.items[prev_prev].clone();
                                NextPrevItem::new(item).render(area, buf);
                            }
                        }
                    }
                    let area = if let Some(area) = state.get_area(FocusListPosition::Prev) {
                        area
                    } else {
                        let area = constrained_centered_area(*rect, 50);
                        state.set_area(area, FocusListPosition::Prev);
                        area
                    };
                    if let Some(previous) = self.prev_index() {
                        let item: FocusListItem<'_> = self.items[previous].clone();
                        NextPrevItem::new(item).render(area, buf);
                    }
                }
                3 => {
                    let area = if let Some(area) = state.get_area(FocusListPosition::Focused) {
                        area
                    } else {
                        let area = constrained_centered_area(*rect, 70);
                        state.set_area(area, FocusListPosition::Focused);
                        area
                    };
                    if let Some(focused) = self.focused_index() {
                        let item: FocusListItem<'_> = self.items[focused].clone();
                        FocusedItem::new(item).render(area, buf);
                    }
                }
                5 => {
                    let area = if let Some(area) = state.get_area(FocusListPosition::NextNext) {
                        area
                    } else {
                        let area = constrained_centered_area(*rect, 50);
                        state.set_area(area, FocusListPosition::NextNext);
                        area
                    };
                    if let Some(next_next) = self.next_next_index() {
                        if let Some(anim_area) = &state.next_next_area {
                            if !anim_area.is_completed() {
                                let item: FocusListItem<'_> = self.items[next_next].clone();
                                NextPrevItem::new(item).render(area, buf);
                            }
                        }
                    }
                    let area = if let Some(area) = state.get_area(FocusListPosition::Next) {
                        area
                    } else {
                        let area = constrained_centered_area(*rect, 50);
                        state.set_area(area, FocusListPosition::Next);
                        area
                    };
                    if let Some(next) = self.next_index() {
                        let item: FocusListItem<'_> = self.items[next].clone();
                        NextPrevItem::new(item).render(area, buf);
                    }
                }
                _ => {}
            });
    }
}

fn constrained_centered_area(area: Rect, max: u16) -> Rect {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Fill(1),
            Constraint::Max(max),
            Constraint::Fill(1),
        ])
        .split(area)[1]
}
