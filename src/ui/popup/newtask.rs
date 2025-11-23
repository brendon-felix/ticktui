use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, BorderType, Borders, Clear},
};
use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::Input;

use crate::{
    app::AppAction,
    tasks::{TaskAction, TaskData},
    ui::{
        UIAction,
        editor::{
            Editor, EditorMode,
            actions::{EditorAction, EditorActions},
            handlers,
        },
        popup::Popup,
        utils,
        viewselector::View,
    },
};

pub struct NewTaskPopup {
    editor: Editor,
    view: View,
    block: Block<'static>,
    tx: UnboundedSender<AppAction>,
}

impl NewTaskPopup {
    pub fn new(view: View, tx: UnboundedSender<AppAction>) -> Self {
        let mut editor = Editor::new().with_single_line();
        editor.set_mode(EditorMode::Insert);
        Self {
            editor,
            view,
            block: Block::new()
                .title("New Task")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
            tx,
        }
    }

    fn submit(&mut self) {
        let content = self.editor.get_content().trim().to_string();
        if !content.is_empty() {
            // let data = TaskData::default().title("Test".into());
            let data = parse(&content);
            let _ = self
                .tx
                .send(AppAction::TaskAction(TaskAction::Create, data));
            let _ = self.tx.send(AppAction::UIAction(UIAction::ClosePopup));
        }
    }
}

impl Popup for NewTaskPopup {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => {
                let _ = self.tx.send(AppAction::UIAction(UIAction::ClosePopup));
            }
            _ => {
                let input: Input = key_event.into();
                let mode = self.editor.get_mode();
                let action_opt = if let Some(pending_action) = self.editor.get_pending_action() {
                    match handlers::handle_pending_action_input(input, pending_action) {
                        Some(action) => Some(action),
                        None => {
                            self.editor.set_pending_action(None);
                            None
                        }
                    }
                } else {
                    handlers::handle_input(input, mode, true)
                };
                if let Some(action) = action_opt {
                    match action {
                        EditorAction::Submit => self.submit(),
                        _ => self.editor.execute_action(action),
                    }
                }
            }
        }
    }

    fn handle_mouse_event(&mut self, _mouse_event: MouseEvent) {}

    fn allow_key_cmd(&self) -> bool {
        self.editor.get_mode() == EditorMode::Normal
    }

    fn draw(&mut self, f: &mut Frame, area: Rect, _last_frame: Instant) {
        let popup_area = utils::centered_area(area, 3, 40);
        f.render_widget(Clear, popup_area);

        let inner_area = self.block.inner(popup_area);
        f.render_widget(&self.block, popup_area);
        self.editor.update_style();
        f.render_widget(&self.editor, inner_area);
    }
}

fn parse(content: &str) -> TaskData {
    let mut data = TaskData::default();
    data = data.title(content.into());
    data
}
