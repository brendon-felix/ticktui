use std::time::Instant;

use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    widgets::{Block, Borders},
};
use tachyonfx::{
    EffectManager, EffectTimer, Interpolation,
    fx::{self},
};
use ticks::tasks::Task;
use tui_textarea::Input;

use crate::ui::{
    composite::{CompositeEditor, CompositeEditorState},
    editor::{
        Editor, EditorMode,
        actions::{EditorAction, EditorActions},
        handlers::{handle_input, handle_pending_action_input},
    },
};

// const SAMPLE_DESCRIPTION: &str = r#"This is a description.
// You can write multiple lines here,
// or edit the content as needed.

// This is another paragraph to demonstrate the editor functionality.
// Next we have a line that is really long ..."#;

pub struct TaskEditor {
    editor: CompositeEditor,
    editor_state: CompositeEditorState,
    effects: EffectManager<()>,
}

impl TaskEditor {
    pub fn new() -> Self {
        let editors = vec![
            Editor::default().with_single_line().with_title("Title"),
            Editor::default().with_single_line().with_title("Date"),
            // .with_validator(validator),
            Editor::default().with_single_line().with_title("Time"),
            // .with_validator(validator),
            Editor::default().with_title("Description"),
        ];
        let editor_state = CompositeEditorState::new(editors.len());
        let editor = CompositeEditor::new(editors).with_constraints(vec![
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(3),
        ]);
        let effects: EffectManager<()> = EffectManager::default();
        // let c = Color::Rgb(25, 25, 25);
        // let timer = EffectTimer::from_ms(20000, Interpolation::ElasticOut);
        // let fx = fx::sweep_in(Motion::LeftToRight, 5, 0, c, timer);
        // effects.add_effect(fx);
        Self {
            editor,
            editor_state,
            effects,
        }
    }

    pub fn deactivate(&mut self) {
        self.editor.set_pending_action(None);
        self.editor.set_active_editor(None);
    }

    pub fn activate(&mut self) {
        self.editor.set_active_editor(Some(0));
    }

    pub fn set_title_content(&mut self, title: &str) {
        self.editor.editors[0].set_content(title);
    }

    pub fn set_due_date_content(&mut self, due_date: &str) {
        self.editor.editors[1].set_content(due_date);
    }

    pub fn set_due_time_content(&mut self, due_date: &str) {
        self.editor.editors[2].set_content(due_date);
    }

    pub fn set_description_content(&mut self, title: &str) {
        self.editor.editors[3].set_content(title);
    }

    pub fn load_task(&mut self, task: &Task) {
        self.set_title_content(&task.title);
        if task.due_date.timestamp() > 0 {
            let due_date_str = task
                .due_date
                .with_timezone(&chrono::Local)
                .format("%m/%d/%Y")
                .to_string();
            self.set_due_date_content(&due_date_str);
            if !task.is_all_day {
                let due_time_str = task
                    .due_date
                    .with_timezone(&chrono::Local)
                    .format("%I:%M %p")
                    .to_string();
                self.set_due_time_content(&due_time_str);
            } else {
                self.set_due_time_content("");
            }
        } else {
            self.set_due_date_content("");
            self.set_due_time_content("");
        }
        self.set_description_content(&task.content);
        self.editor_state
            .get_sub_areas()
            .iter()
            .map(|area| {
                let inner = Block::default().borders(Borders::ALL).inner(area.clone());
                // let bg = Color::Rgb(25, 25, 25);
                let timer = EffectTimer::from_ms(200, Interpolation::Linear);
                fx::coalesce(timer).with_area(inner)
                // fx::sweep_in(Motion::RightToLeft, 5, 0, bg, timer).with_area(inner)
            })
            .for_each(|fx| self.effects.add_effect(fx));
    }

    pub fn is_in_insert_mode(&self) -> bool {
        if let Some(mode) = self.editor.get_mode() {
            match mode {
                EditorMode::Insert | EditorMode::Visual(_) => true,
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        let input: Input = key_event.into();
        if let Some(mode) = self.editor.get_mode() {
            let action_opt = if let Some(pending_action) = self.editor.get_pending_action() {
                match handle_pending_action_input(input, pending_action) {
                    Some(action) => Some(action),
                    None => {
                        self.editor.set_pending_action(None);
                        None
                    }
                }
            } else {
                handle_input(input, mode)
            };
            match action_opt {
                Some(action) => match action {
                    EditorAction::ApplyInput(_) => self.editor.execute_action(action),
                    _ => self.editor.execute_action(action),
                },
                None => {}
            }
        }
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect, last_frame: Instant) {
        f.render_stateful_widget(&mut self.editor, area, &mut self.editor_state);
        let elapsed = last_frame.elapsed();
        self.effects
            .process_effects(elapsed.into(), f.buffer_mut(), area);
    }
}
