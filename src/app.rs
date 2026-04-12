use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::{
    db::Db,
    debug,
    tasks::{Task, TaskAction, TaskData},
    term::{self, AppTerminal},
    ui::{AppUI, UIAction},
};

#[derive(Debug, Clone)]
pub enum AppAction {
    Tick,
    Render(Instant),
    Resize(u16, u16),
    Quit,
    RefreshData,
    UpdateCache,
    TaskAction(TaskAction, TaskData),
    UIAction(UIAction),
    MultiAction(Vec<AppAction>),
    AfterNTicks(u32, Box<AppAction>),
}

pub struct PendingAction {
    ticks: u32,
    action: AppAction,
}

pub struct App {
    db: Arc<Db>,
    cached_tasks: Arc<Vec<Arc<Task>>>,
    pending_tasks: Arc<Mutex<Option<Vec<Task>>>>,
    pending_action: Option<PendingAction>,
    ti: AppTerminal,
    ui: AppUI,
    quitting: bool,
    tx: UnboundedSender<AppAction>,
    rx: UnboundedReceiver<AppAction>,
}

impl App {
    pub fn new(db: Arc<Db>) -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        let cached_tasks = Arc::new(Vec::new());
        let pending_tasks = Arc::new(Mutex::new(None));
        let pending_action = None;
        let ti = AppTerminal::new()?;
        let ui = AppUI::new(tx.clone());
        let quitting = false;
        crate::debug::init_debug_sender(tx.clone());
        Ok(Self {
            db,
            cached_tasks,
            pending_tasks,
            pending_action,
            ti,
            ui,
            quitting,
            tx,
            rx,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let tx = self.tx.clone();
        self.ti.enter()?;
        self.tx.send(AppAction::RefreshData)?;

        // Start optional Postgres sync loop if DATABASE_URL is set
        if let Ok(pg_url) = std::env::var("DATABASE_URL") {
            let conn = Arc::clone(&self.db.conn);
            let sync_tx = self.tx.clone();
            tokio::spawn(crate::db::sync::run_sync_loop(conn, pg_url, sync_tx));
        }

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
        let conn = Arc::clone(&self.db.conn);
        let pending = Arc::clone(&self.pending_tasks);
        tokio::spawn(async move {
            // If no Postgres sync is configured, soft-deleted rows will never be
            // pushed anywhere — hard-delete them immediately as a cleanup step.
            if std::env::var("DATABASE_URL").is_err() {
                if let Ok(dirty) = crate::db::local::fetch_dirty_tasks(Arc::clone(&conn)).await {
                    for task in dirty.iter().filter(|t| t.deleted) {
                        let _ =
                            crate::db::local::hard_delete_task(Arc::clone(&conn), task.id.clone())
                                .await;
                    }
                }
            }
            match crate::db::local::fetch_all_tasks_including_completed(conn).await {
                Ok(tasks) => {
                    if let Ok(mut guard) = pending.lock() {
                        *guard = Some(tasks);
                    }
                    let _ = tx.send(AppAction::UpdateCache);
                }
                Err(e) => {
                    debug!("{}", &e.to_string());
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
            KeyCode::Char('q') if self.ui.allow_key_cmd() => {
                tx.send(AppAction::UIAction(UIAction::Confirm(
                    ratatui::text::Text::from("Are you sure you want to quit?"),
                    Box::new(AppAction::Quit),
                )))?
            }
            KeyCode::Char('r')
                if key_event.modifiers == KeyModifiers::CONTROL && self.ui.allow_key_cmd() =>
            {
                tx.send(AppAction::RefreshData)?
            }
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                tx.send(AppAction::Quit)?
            }
            _ => self.ui.handle_key_event(key_event, tx),
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
                self.ui.next_tick();
                if let Some(pending) = &mut self.pending_action {
                    if pending.ticks > 0 {
                        pending.ticks -= 1;
                    } else {
                        let act = pending.action.clone();
                        self.pending_action = None;
                        self.execute_action(act, tx)?;
                    }
                }
            }
            AppAction::Render(last_frame) => self.render(last_frame)?,
            AppAction::Resize(w, h) => {
                self.ti.resize(w, h)?;
                self.ui.reset_areas();
            }
            AppAction::Quit => self.quitting = true,
            AppAction::RefreshData => self.refresh_tasks(tx.clone()),
            AppAction::UpdateCache => self.update_cache(),
            AppAction::TaskAction(action, data) => self.execute_task_action(action, data, tx),
            AppAction::UIAction(action) => self.ui.execute_action(action, tx),
            AppAction::MultiAction(actions) => {
                for act in actions {
                    self.execute_action(act, tx)?;
                }
            }
            AppAction::AfterNTicks(n_ticks, action) => {
                self.pending_action = Some(PendingAction {
                    ticks: n_ticks,
                    action: *action,
                });
            }
        }
        Ok(())
    }

    fn execute_task_action(
        &mut self,
        action: TaskAction,
        data: TaskData,
        tx: &UnboundedSender<AppAction>,
    ) {
        let conn = Arc::clone(&self.db.conn);
        let tx_clone = tx.clone();
        let _ = tokio::spawn(async move {
            let result: anyhow::Result<()> = match action {
                TaskAction::Create => crate::db::local::create_task(conn, data).await.map(|_| ()),
                TaskAction::Edit => crate::db::local::edit_task(conn, data).await,
                TaskAction::Complete => {
                    let id = data.task_id.unwrap_or_default();
                    crate::db::local::complete_task(conn, id).await
                }
                TaskAction::Delete => {
                    let id = data.task_id.unwrap_or_default();
                    crate::db::local::delete_task(conn, id).await
                }
            };
            match result {
                Ok(()) => {
                    let _ = tx_clone.send(AppAction::RefreshData);
                }
                Err(e) => {
                    debug!("{}", e.to_string());
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
}
