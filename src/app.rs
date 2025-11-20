use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use ticks::{
    TickTick,
    projects::ProjectID,
    tasks::{Task, TaskID},
};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::{
    tasks::{self, TaskAction, fetch_all_tasks},
    term::{self, AppTerminal},
    ui::AppUI,
};

#[derive(Debug, Clone)]
pub enum AppAction {
    Tick,
    Render(Instant),
    Resize(u16, u16),
    Quit,
    Debug(String),
    RefreshData,
    UpdateCache,
    TaskAction(ProjectID, TaskID, TaskAction),
    Confirm(Box<AppAction>),
    ClosePopup,
    MultiAction(Vec<AppAction>),
}

pub struct App {
    client: Arc<TickTick>,
    cached_tasks: Arc<Vec<Arc<Task>>>,
    pending_tasks: Arc<Mutex<Option<Vec<Task>>>>,
    ti: AppTerminal,
    ui: AppUI,
    quitting: bool,
    tick_count: u32,
    tx: UnboundedSender<AppAction>,
    rx: UnboundedReceiver<AppAction>,
}

impl App {
    pub fn new(client: Arc<TickTick>) -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        let cached_tasks = Arc::new(Vec::new());
        let pending_tasks = Arc::new(Mutex::new(None));
        let ti = AppTerminal::new()?;
        let ui = AppUI::new(tx.clone());
        let quitting = false;
        let tick_count = 0;
        Ok(Self {
            client,
            cached_tasks,
            pending_tasks,
            ti,
            ui,
            quitting,
            tick_count,
            tx,
            rx,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let tx = self.tx.clone();
        self.ti.enter()?;
        self.tx.send(AppAction::RefreshData)?;

        loop {
            if let Some(event) = self.ti.next().await {
                self.handle_event(event, &tx)?;
            }

            while let Ok(action) = self.rx.try_recv() {
                self.execute_action(action, &tx)?;
            }

            if self.quitting {
                break;
            }
        }

        self.ti.exit()?;
        Ok(())
    }

    fn refresh_tasks(&mut self, tx: UnboundedSender<AppAction>) {
        let client = Arc::clone(&self.client);
        let pending = Arc::clone(&self.pending_tasks);
        tokio::spawn(async move {
            match fetch_all_tasks(&client).await {
                Ok(tasks) => {
                    // Store the tasks in pending storage
                    if let Ok(mut guard) = pending.lock() {
                        *guard = Some(tasks);
                    }
                    let _ = tx.send(AppAction::UpdateCache);
                }
                Err(e) => {
                    let _ = tx.send(AppAction::Debug(e.to_string()));
                }
            }
        });
    }

    fn update_cache(&mut self) {
        let tasks_opt = if let Ok(mut guard) = self.pending_tasks.lock() {
            guard.take()
        } else {
            None
        };

        if let Some(tasks) = tasks_opt {
            self.cached_tasks = Arc::new(tasks.into_iter().map(Arc::new).collect());
            self.ui.update_tasks(Arc::clone(&self.cached_tasks));
        }
    }

    fn handle_event(&mut self, event: term::Event, tx: &UnboundedSender<AppAction>) -> Result<()> {
        match event {
            // term::Event::Quit => tx.send(AppAction::Quit)?,
            term::Event::Tick => tx.send(AppAction::Tick)?,
            term::Event::Render(last) => tx.send(AppAction::Render(last))?,
            term::Event::Resize(w, h) => tx.send(AppAction::Resize(w, h))?,
            term::Event::Key(key) => self.handle_key_event(key, tx)?,
            term::Event::Mouse(mouse) => self.handle_mouse_event(mouse, tx)?,
            term::Event::Paste(_content) => {}
            _ => {}
        }
        Ok(())
    }

    fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
        tx: &UnboundedSender<AppAction>,
    ) -> Result<()> {
        match key_event.code {
            KeyCode::Char('q') => {
                if !self.ui.allow_quit() {
                    self.ui.handle_key_event(key_event);
                } else {
                    tx.send(AppAction::Quit)?;
                }
            }
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                tx.send(AppAction::Quit)?
            }
            _ => self.ui.handle_key_event(key_event),
        }
        Ok(())
    }

    fn handle_mouse_event(
        &mut self,
        mouse_event: MouseEvent,
        _tx: &UnboundedSender<AppAction>,
    ) -> Result<()> {
        self.ui.handle_mouse_event(mouse_event);
        Ok(())
    }

    fn execute_action(&mut self, action: AppAction, tx: &UnboundedSender<AppAction>) -> Result<()> {
        match action {
            AppAction::Tick => {
                self.tick_count += 1;
                // if self.tick_count >= 120 {
                //     self.tick_count = 0;
                //     tx.send(AppAction::RefreshData)?;
                // }
                self.ui.next_tick();
            }
            AppAction::Render(last_frame) => self.render(last_frame)?,
            AppAction::Resize(w, h) => {
                self.ti.resize(w, h)?;
                self.ui.reset_areas();
            }
            AppAction::Quit => self.quitting = true,
            AppAction::RefreshData => self.refresh_tasks(tx.clone()),
            AppAction::UpdateCache => self.update_cache(),
            AppAction::TaskAction(p_id, t_id, action) => {
                self.execute_task_action(p_id, t_id, action)
            }
            AppAction::Debug(msg) => self.ui.debug(msg),
            AppAction::Confirm(action) => self.ui.confirm(*action),
            AppAction::ClosePopup => self.ui.close_popup(),
            AppAction::MultiAction(actions) => {
                for act in actions {
                    self.execute_action(act, tx)?;
                }
            }
        }
        Ok(())
    }

    fn execute_task_action(&mut self, project_id: ProjectID, task_id: TaskID, action: TaskAction) {
        let client = Arc::clone(&self.client);
        tokio::spawn(async move {
            match action {
                TaskAction::Complete => {
                    let _ = tasks::complete_task(&client, &project_id, &task_id).await;
                }
                TaskAction::Delete => {
                    let _ = tasks::delete_task(&client, &project_id, &task_id).await;
                }
            }
        });
    }

    fn render(&mut self, last_frame: Instant) -> Result<()> {
        self.ti.draw(|f| {
            self.ui.draw(f, f.area(), last_frame);
        })?;
        Ok(())
    }

    // fn error(&mut self, _message: String) {}
}
