pub mod actions;
pub mod handlers;
pub mod helpers;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Widget, WidgetRef, block::Position},
};
use std::fmt;
use tui_textarea::{CursorMove, TextArea};

use actions::EditorPendingAction;

use crate::ui::editor::{
    actions::{EditorAction, EditorActions},
    helpers::{create_block, cursor_style},
};

#[derive(Debug, Clone, Copy)]
pub enum TextObject {
    Char,
    WordInner,
    WordAround,
    Line,
    ParagraphInner,
    ParagraphAround,
    Selection,
    To(CursorMove),
}

#[derive(Debug, Clone, Copy)]
pub enum TextObjectModifier {
    Inner,
    Around,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualMode {
    Char,
    Line,
    // Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    Insert,
    Replace,
    Visual(VisualMode),
}

impl fmt::Display for EditorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Insert => write!(f, "INSERT"),
            Self::Replace => write!(f, "REPLACE"),
            Self::Visual(_) => write!(f, "VISUAL"),
        }
    }
}

#[allow(dead_code)]
pub struct EditorState {
    mode: EditorMode,
    pending_action: Option<EditorPendingAction>,
    yank_type: Option<TextObject>,
}

#[allow(dead_code)]
pub struct Editor {
    title: Option<String>,
    state: EditorState,
    textarea: TextArea<'static>,
    single_line: bool,
    validator: Option<Box<dyn Fn(&TextArea) -> bool>>,
    current_block: Option<Block<'static>>,
    last_area_pos: Option<Position>,
}

impl Default for Editor {
    fn default() -> Self {
        let state = EditorState {
            mode: EditorMode::Normal,
            pending_action: None,
            yank_type: None,
        };
        let mut textarea = TextArea::default();
        textarea.set_selection_style(Style::default().add_modifier(Modifier::REVERSED));
        // textarea.set_cursor_line_style(Style::default().bg(Color::Rgb(50, 50, 50)));
        textarea.set_cursor_line_style(Style::default());
        Self {
            title: None,
            state,
            textarea,
            single_line: false,
            validator: None,
            current_block: None,
            last_area_pos: None,
        }
    }
}

#[allow(dead_code)]
impl Editor {
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_single_line(mut self) -> Self {
        self.single_line = true;
        self
    }

    pub fn with_validator<F>(mut self, validator: F) -> Self
    where
        F: 'static + Fn(&TextArea) -> bool,
    {
        self.validator = Some(Box::new(validator));
        self
    }

    pub fn with_content(mut self, content: &str) -> Self {
        self.textarea.insert_str(content);
        self
    }

    pub fn with_placeholder(mut self, placeholder: &str) -> Self {
        self.textarea.set_placeholder_text(placeholder);
        self
    }

    pub fn with_input_mask(mut self) -> Self {
        self.textarea.set_mask_char('*');
        self
    }

    pub fn with_block(mut self, block: Block<'static>) -> Self {
        self.current_block = Some(block.clone());
        self.textarea.set_block(block);
        self
    }

    pub fn with_cursor_line_style(mut self, style: Style) -> Self {
        self.textarea.set_cursor_line_style(style);
        self
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.textarea.set_style(style);
        self
    }

    pub fn with_cursor_style(mut self, style: Style) -> Self {
        self.textarea.set_cursor_style(style);
        self
    }

    pub fn set_content(&mut self, content: &str) {
        self.textarea.reset_with_content(content);
    }

    pub fn set_block(&mut self, block: Block<'static>) {
        self.current_block = Some(block.clone());
        self.textarea.set_block(block);
    }

    pub fn set_style(&mut self, style: Style) {
        self.textarea.set_style(style);
    }

    pub fn set_cursor_style(&mut self, style: Style) {
        self.textarea.set_cursor_style(style);
    }

    pub fn set_cursor_line_style(&mut self, style: Style) {
        self.textarea.set_cursor_style(style);
    }

    pub fn set_desired_column(&mut self, col: usize) {
        self.textarea.set_desired_column(col);
    }

    pub fn set_cursor_pos(mut self, row: u16, col: u16) -> Self {
        self.textarea.move_cursor(CursorMove::Jump(row, col));
        self
    }

    pub fn set_style_active(&mut self) {
        let is_active = true;
        self.set_cursor_style(cursor_style(self.get_mode(), is_active));
        self.set_style(Style::default());
        let borders = Borders::ALL;
        self.set_block(create_block(self.get_title(), is_active, borders))
    }

    pub fn set_style_inactive(&mut self) {
        let is_active = false;
        self.set_cursor_style(cursor_style(self.get_mode(), is_active));
        self.set_style(Style::default().add_modifier(Modifier::DIM));
        let borders = Borders::ALL;
        self.set_block(create_block(self.get_title(), is_active, borders))
    }

    pub fn insert_str(&mut self, s: String) {
        let _ = self.textarea.insert_str(s);
    }

    pub fn is_single_line(&self) -> bool {
        self.single_line
    }

    pub fn get_title(&self) -> Option<String> {
        self.title.clone()
    }

    pub fn get_mode(&self) -> EditorMode {
        self.state.mode
    }

    pub fn get_cursor_pos(&self) -> (usize, usize) {
        self.textarea.cursor()
    }

    pub fn get_desired_column(&self) -> usize {
        self.textarea.get_desired_column()
    }

    pub fn get_lines(&self) -> &[String] {
        self.textarea.lines()
    }

    pub fn is_cursor_at_line_end(&self) -> bool {
        let (row, col) = self.textarea.cursor();
        if let Some(line) = self.textarea.lines().get(row) {
            col >= line.len()
        } else {
            false
        }
    }

    pub fn is_cursor_at_line_start(&self) -> bool {
        let (_row, col) = self.textarea.cursor();
        col == 0
    }

    pub fn is_cursor_on_first_line(&self) -> bool {
        let (row, _col) = self.textarea.cursor();
        row == 0
    }

    pub fn is_cursor_on_last_line(&self) -> bool {
        let (row, _col) = self.textarea.cursor();
        row + 1 == self.textarea.lines().len()
    }
}

fn select_current_word(textarea: &mut TextArea, modifier: TextObjectModifier) -> (usize, usize) {
    let (current_row, current_col) = textarea.cursor();
    textarea.move_cursor(CursorMove::WordBack);
    textarea.start_selection();
    match modifier {
        TextObjectModifier::Inner => {
            textarea.move_cursor(CursorMove::WordEnd);
            textarea.move_cursor(CursorMove::Right);
        }
        TextObjectModifier::Around => {
            textarea.move_cursor(CursorMove::WordForward);
        }
    }
    (current_row, current_col)
}
fn select_current_line(textarea: &mut TextArea) -> (usize, usize) {
    let (current_row, current_col) = textarea.cursor();
    let total_lines = textarea.lines().len();

    if current_row + 1 == total_lines && total_lines > 1 {
        // Last line case: select from end of previous line to end of current line
        textarea.move_cursor(CursorMove::Up);
        textarea.move_cursor(CursorMove::End);
        textarea.start_selection();
        textarea.move_cursor(CursorMove::Down);
        textarea.move_cursor(CursorMove::End);
    } else {
        // Normal case: select entire line including newline
        textarea.move_cursor(CursorMove::Head);
        textarea.start_selection();
        let cursor = textarea.cursor();
        textarea.move_cursor(CursorMove::Down);
        if cursor == textarea.cursor() {
            textarea.move_cursor(CursorMove::End); // At the last line, move to end of the line instead
        }
    }
    (current_row, current_col)
}
fn select_current_paragraph(
    textarea: &mut TextArea,
    _modifier: TextObjectModifier,
) -> (usize, usize) {
    let (current_row, current_col) = textarea.cursor();
    (current_row, current_col)
}

impl EditorActions for Editor {
    #[rustfmt::skip]
    fn execute_action(&mut self, action: EditorAction) {
        let mut pending = false;
        match action {
            EditorAction::SetMode(mode) => {
                match mode {
                    EditorMode::Normal => {
                        self.textarea.cancel_selection();
                    }
                    EditorMode::Visual(vmode) => {
                        match vmode {
                            VisualMode::Char =>  self.textarea.start_selection(),
                            VisualMode::Line => self.textarea.start_line_selection(),
                        }
                    }
                    EditorMode::Insert | EditorMode::Replace => {}
                }
                self.state.mode = mode;
                self.textarea
                    .set_cursor_style(cursor_style(self.state.mode, true));
            }
            EditorAction::MoveCursor(mvmt) => {
                match mvmt {
                    _ => self.textarea.move_cursor(mvmt),
                }
            }
            EditorAction::Insert(obj) => {
                match obj {
                    TextObject::Line => {
                        if self.single_line {
                            return;
                        }
                        let _ = self.textarea.insert_newline();
                    }
                    _ => {}
                }
            }
            EditorAction::ApplyInput(input) => {
                if self.single_line {
                    match input.key {
                        tui_textarea::Key::Enter => return,
                        _ => {}
                    }
                }
                let _ = self.textarea.input(input);
            }
            EditorAction::Delete(obj) => {
                self.state.yank_type = Some(obj);
                match obj {
                    TextObject::Char => {
                        self.textarea.start_selection();
                        self.textarea.move_cursor(CursorMove::Forward);
                        self.textarea.cut();
                    }
                    TextObject::WordInner => {
                        let _ = select_current_word(&mut self.textarea, TextObjectModifier::Inner);
                        self.textarea.cut();
                        // self.textarea.move_cursor(CursorMove::Jump(current_row as u16, current_col as u16));
                    }
                    TextObject::WordAround => {
                        let _ = select_current_word(&mut self.textarea, TextObjectModifier::Around);
                        self.textarea.cut();
                        // self.textarea.move_cursor(CursorMove::Jump(current_row as u16, current_col as u16));
                    }
                    TextObject::Line => {
                        let (current_row, current_col) = select_current_line(&mut self.textarea);
                        self.textarea.cut();
                        self.textarea.move_cursor(CursorMove::Jump(current_row as u16, current_col as u16));
                    }
                    TextObject::Selection => {
                        self.textarea.cut();
                    }
                    TextObject::To(mvmt) => {
                        self.textarea.start_selection();
                        self.textarea.move_cursor(mvmt);
                        self.textarea.cut();
                    }
                    _ => {}
                }
            },
            EditorAction::Select(obj) => {
                match obj {
                    TextObject::WordInner => {
                        let _ = select_current_word(&mut self.textarea, TextObjectModifier::Inner);
                    }
                    TextObject::WordAround => {
                        let _ = select_current_word(&mut self.textarea, TextObjectModifier::Around);
                    }
                    TextObject::ParagraphInner => {
                        let _ = select_current_paragraph(&mut self.textarea, TextObjectModifier::Inner);
                    }
                    TextObject::ParagraphAround => {
                        let _ = select_current_paragraph(&mut self.textarea, TextObjectModifier::Inner);
                    }
                    _ => {}
                }
                self.state.mode = EditorMode::Visual(VisualMode::Char);
                self.textarea
                    .set_cursor_style(cursor_style(self.state.mode, true));
            },
            EditorAction::Yank(obj) => {
                self.state.yank_type = Some(obj);
                match obj {
                    TextObject::Line => {
                        let (current_row, current_col) = select_current_line(&mut self.textarea);
                        self.textarea.copy();
                        self.textarea.move_cursor(CursorMove::Jump(current_row as u16, current_col as u16));
                    }
                    TextObject::Selection => {
                        if self.state.mode == EditorMode::Visual(VisualMode::Line) {
                            self.state.yank_type = Some(TextObject::Line);
                        }
                        self.textarea.copy();
                    }
                    _ => {}
                }
            }
            EditorAction::ReplaceChar(c) => {
                self.textarea.start_selection();
                self.textarea.move_cursor(CursorMove::Forward);
                self.textarea.set_yank_text(c);
                self.textarea.insert_char(c);
                self.textarea.move_cursor(CursorMove::Back);
            }
            // EditorAction::Copy => {
            //     self.textarea.copy();
            // }
            EditorAction::Paste => {
                match self.state.yank_type {
                    Some(TextObject::Line) => {
                        if self.single_line {
                            return;
                        }
                        let yanked = &self.textarea.yank_text();
                        self.textarea.set_yank_text(yanked.trim_end());
                        self.textarea.move_cursor(CursorMove::End);
                        let _ = self.textarea.insert_newline();
                        self.textarea.paste();
                        self.textarea.move_cursor(CursorMove::Head);
                    }
                    _ => {
                        self.textarea.move_cursor(CursorMove::Forward);
                        self.textarea.paste();
                    }
                }
            }
            EditorAction::Undo => {
                self.textarea.undo();
            }
            EditorAction::Redo => {
                self.textarea.redo();
            }
            // EditorAction::Command(cmd) => match cmd {
            //     EditorCommand::Submit => {}
            // },
            EditorAction::Pending(p) => {
                pending = true;
                self.state.pending_action = Some(p);
            }
            EditorAction::MultiAction(actions) => {
                for act in actions {
                    self.execute_action(act);
                }
            }
            EditorAction::Submit => {
                // handled externally
            }
        }

        if !pending {
            self.state.pending_action = None;
        }
        // match self.state.mode {
        //     EditorMode::Normal => match input {
        //         Input {
        //             key: Key::Char('i'),
        //             ..
        //         } => {
        //             self.state.mode = EditorMode::Insert;
        //             self.textarea
        //                 .set_cursor_style(cursor_style(self.state.mode, true));
        //         }
        //         _ => {}
        //     },
        //     EditorMode::Insert => match input {
        //         Input { key: Key::Esc, .. } => {
        //             self.state.mode = EditorMode::Normal;
        //             self.textarea
        //                 .set_cursor_style(cursor_style(self.state.mode, true));
        //         }
        //         input => {
        //             self.textarea.input(input);
        //         }
        //     },
        //     EditorMode::Visual(_) => match input {
        //         Input { key: Key::Esc, .. } => {
        //             self.state.mode = EditorMode::Normal;
        //             self.textarea
        //                 .set_cursor_style(cursor_style(self.state.mode, true));
        //         }
        //         _ => {}
        //     },
        // }
    }
    fn set_pending_action(&mut self, pending: Option<EditorPendingAction>) {
        self.state.pending_action = pending;
    }
    fn get_pending_action(&mut self) -> Option<EditorPendingAction> {
        self.state.pending_action
    }
}

// impl AppWidget for Editor {
//     fn set_widget_style(&mut self, style: WidgetStyle) {
//         match style {
//             WidgetStyle::Active => {
//                 let is_active = true;
//                 self.set_cursor_style(cursor_style(self.get_mode(), is_active));
//                 self.set_style(Style::default());
//                 let borders = Borders::ALL;
//                 self.set_block(create_block(self.get_title(), is_active, borders))
//             }
//             WidgetStyle::Inactive => {
//                 let is_active = false;
//                 self.set_cursor_style(cursor_style(self.get_mode(), is_active));
//                 self.set_style(Style::default().add_modifier(Modifier::DIM));
//                 let borders = Borders::ALL;
//                 self.set_block(create_block(self.get_title(), is_active, borders))
//             }
//         }
//     }

//     fn on_click(&mut self, pos: Position) {
//         if let Some(area_pos) = self.last_area_pos {
//             let local = Position::new(
//                 pos.x.saturating_sub(area_pos.x),
//                 pos.y.saturating_sub(area_pos.y),
//             );
//             let (x, y) = if let Some(_block) = &self.current_block {
//                 (local.x.saturating_sub(1), local.y.saturating_sub(1))
//             } else {
//                 local.into()
//             };
//             self.textarea.move_cursor(CursorMove::Jump(y, x));
//         }
//     }

//     fn set_last_area_pos(&mut self, area_pos: Position) {
//         self.last_area_pos = Some(area_pos);
//     }
// }

impl Widget for Editor {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.textarea.render(area, buf);
    }
}

impl Widget for &mut Editor {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.textarea.render(area, buf);
    }
}

impl WidgetRef for Editor {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        self.textarea.clone().render(area, buf);
    }
}
