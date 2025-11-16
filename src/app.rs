use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use crate::{
    tasks::fetch_all_tasks,
    term::{self, AppTerminal},
    ui::AppUI,
};

use anyhow::{Error, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ticks::{TickTick, tasks::Task};
use tokio::sync::mpsc::{self, UnboundedSender};

enum Action {
    Tick,
    Render(Instant),
    Resize(u16, u16),
    Quit,
    Error(Error),
    RefreshTasks,
    UpdateCache,
}

pub struct App {
    client: Arc<TickTick>,
    cached_tasks: Vec<Arc<Task>>,
    pending_tasks: Arc<Mutex<Option<Vec<Task>>>>,
    ti: AppTerminal,
    ui: AppUI,
    quitting: bool,
    tick_count: u32,
}

impl App {
    pub fn new(client: Arc<TickTick>) -> Result<Self> {
        let cached_tasks = Vec::new();
        let pending_tasks = Arc::new(Mutex::new(None));
        let ti = AppTerminal::new()?;
        let ui = AppUI::new();
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
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        self.ti.enter()?;
        tx.send(Action::RefreshTasks)?;

        loop {
            if let Some(event) = self.ti.next().await {
                self.handle_event(event, &tx)?;
            }

            while let Ok(action) = rx.try_recv() {
                self.execute_action(action, &tx)?;
            }

            if self.quitting {
                break;
            }
        }

        self.ti.exit()?;
        Ok(())
    }

    fn refresh_tasks(&mut self, tx: UnboundedSender<Action>) {
        let client = Arc::clone(&self.client);
        let pending = Arc::clone(&self.pending_tasks);
        tokio::spawn(async move {
            match fetch_all_tasks(&client).await {
                Ok(tasks) => {
                    // Store the tasks in pending storage
                    if let Ok(mut guard) = pending.lock() {
                        *guard = Some(tasks);
                    }
                    let _ = tx.send(Action::UpdateCache);
                }
                Err(e) => {
                    let _ = tx.send(Action::Error(e));
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
            self.cached_tasks = tasks.into_iter().map(Arc::new).collect();
            self.ui.update_tasks(self.cached_tasks.clone());
        }
    }

    fn handle_event(&mut self, event: term::Event, tx: &UnboundedSender<Action>) -> Result<()> {
        match event {
            // term::Event::Quit => tx.send(Action::Quit)?,
            term::Event::Tick => tx.send(Action::Tick)?,
            term::Event::Render(last) => tx.send(Action::Render(last))?,
            term::Event::Resize(w, h) => tx.send(Action::Resize(w, h))?,
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
        tx: &UnboundedSender<Action>,
    ) -> Result<()> {
        match key_event.code {
            KeyCode::Char('q') => {
                if self.ui.is_in_insert_mode() {
                    self.ui.handle_key_event(key_event);
                } else {
                    tx.send(Action::Quit)?;
                }
            }
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                tx.send(Action::Quit)?
            }
            _ => self.ui.handle_key_event(key_event),
        }
        Ok(())
    }

    fn handle_mouse_event(
        &mut self,
        mouse_event: MouseEvent,
        _tx: &UnboundedSender<Action>,
    ) -> Result<()> {
        self.ui.handle_mouse_event(mouse_event);
        Ok(())
    }

    fn execute_action(&mut self, action: Action, tx: &UnboundedSender<Action>) -> Result<()> {
        match action {
            Action::Tick => {
                self.tick_count += 1;
                if self.tick_count >= 60 {
                    self.tick_count = 0;
                    tx.send(Action::RefreshTasks)?;
                }
            }
            Action::Render(last_frame) => self.render(last_frame)?,
            Action::Resize(w, h) => self.ti.resize(w, h)?,
            Action::Quit => self.quitting = true,
            Action::RefreshTasks => self.refresh_tasks(tx.clone()),
            Action::UpdateCache => self.update_cache(),
            Action::Error(_e) => {}
        }
        Ok(())
    }

    fn render(&mut self, last_frame: Instant) -> Result<()> {
        self.ti.draw(|f| {
            self.ui.draw(f, f.area(), last_frame);
        })?;
        Ok(())
    }

    // fn error(&mut self, _message: String) {}
}
