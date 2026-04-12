use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, params};
use std::sync::{Arc, Mutex};
use tokio::task;
use uuid::Uuid;

use crate::tasks::{Task, TaskData};

// ---------------------------------------------------------------------------
// Row mapping helper
// ---------------------------------------------------------------------------

fn row_to_task(row: &Row) -> rusqlite::Result<Task> {
    let due_date_str: Option<String> = row.get(4)?;
    let due_date = due_date_str
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let synced_at_str: Option<String> = row.get(10)?;
    let synced_at = synced_at_str
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let updated_at_str: String = row.get(9)?;
    let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
        .unwrap_or_default()
        .with_timezone(&Utc);

    let is_all_day_int: i64 = row.get(8)?;
    let deleted_int: i64 = row.get::<_, i64>(12).unwrap_or(0);

    Ok(Task {
        id: row.get(0)?,
        project_id: row.get(1)?,
        title: row.get(2)?,
        content: row.get(3)?,
        due_date,
        priority: row.get(5)?,
        repeat_flag: row.get(6)?,
        status: row.get(7)?,
        is_all_day: is_all_day_int != 0,
        sort_order: row.get::<_, i64>(11).unwrap_or(0),
        updated_at,
        synced_at,
        deleted: deleted_int != 0,
    })
}

// ---------------------------------------------------------------------------
// Read
// ---------------------------------------------------------------------------

/// Fetch all non-completed tasks, sorted by due_date ASC (NULLs last).
pub async fn fetch_all_tasks(conn: Arc<Mutex<Connection>>) -> Result<Vec<Task>> {
    task::spawn_blocking(move || {
        let guard = conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        let mut stmt = guard.prepare(
            "SELECT id, project_id, title, content, due_date, priority, repeat_flag,
                    status, is_all_day, updated_at, synced_at, sort_order, deleted
             FROM tasks
             WHERE status != 2 AND deleted = 0
             ORDER BY CASE WHEN due_date IS NULL THEN 1 ELSE 0 END,
                      due_date ASC",
        )?;
        let tasks = stmt
            .query_map([], row_to_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(tasks)
    })
    .await?
}

/// Fetch ALL tasks including completed ones, sorted by due_date ASC (NULLs last).
pub async fn fetch_all_tasks_including_completed(
    conn: Arc<Mutex<Connection>>,
) -> Result<Vec<Task>> {
    task::spawn_blocking(move || {
        let guard = conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        let mut stmt = guard.prepare(
            "SELECT id, project_id, title, content, due_date, priority, repeat_flag,
                    status, is_all_day, updated_at, synced_at, sort_order, deleted
             FROM tasks
             WHERE deleted = 0
             ORDER BY CASE WHEN due_date IS NULL THEN 1 ELSE 0 END,
                      due_date ASC",
        )?;
        let tasks = stmt
            .query_map([], row_to_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(tasks)
    })
    .await?
}

/// Fetch all tasks that have not yet been synced to the remote (synced_at IS NULL).
pub async fn fetch_dirty_tasks(conn: Arc<Mutex<Connection>>) -> Result<Vec<Task>> {
    task::spawn_blocking(move || {
        let guard = conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        let mut stmt = guard.prepare(
            "SELECT id, project_id, title, content, due_date, priority, repeat_flag,
                    status, is_all_day, updated_at, synced_at, sort_order, deleted
             FROM tasks
             WHERE synced_at IS NULL",
        )?;
        let tasks = stmt
            .query_map([], row_to_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(tasks)
    })
    .await?
}

// ---------------------------------------------------------------------------
// Write
// ---------------------------------------------------------------------------

/// Insert a new task from TaskData, returning the generated id.
pub async fn create_task(conn: Arc<Mutex<Connection>>, data: TaskData) -> Result<String> {
    task::spawn_blocking(move || {
        let id = Uuid::new_v4().to_string();
        let project_id = data.project_id.unwrap_or_else(|| "inbox".to_string());
        let title = data.title.unwrap_or_default();
        let content = data.content.unwrap_or_default();
        let due_date = data.due_date.map(|dt| dt.to_rfc3339());
        let priority = data.priority.unwrap_or(0);
        let repeat_flag = data.repeat_flag.unwrap_or_default();

        let guard = conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        guard.execute(
            "INSERT INTO tasks (id, project_id, title, content, due_date, priority,
                                repeat_flag, status, is_all_day, updated_at, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 0,
                     strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), NULL)",
            params![
                id,
                project_id,
                title,
                content,
                due_date,
                priority,
                repeat_flag
            ],
        )?;
        Ok(id)
    })
    .await?
}

/// Update an existing task by id using the fields present in TaskData.
pub async fn edit_task(conn: Arc<Mutex<Connection>>, data: TaskData) -> Result<()> {
    task::spawn_blocking(move || {
        let task_id = data
            .task_id
            .clone()
            .ok_or_else(|| anyhow!("edit_task: task_id is required"))?;

        let guard = conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;

        if let Some(title) = &data.title {
            guard.execute(
                "UPDATE tasks SET title = ?1,
                                  updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                                  synced_at = NULL
                 WHERE id = ?2",
                params![title, task_id],
            )?;
        }
        if let Some(content) = &data.content {
            guard.execute(
                "UPDATE tasks SET content = ?1,
                                  updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                                  synced_at = NULL
                 WHERE id = ?2",
                params![content, task_id],
            )?;
        }
        if let Some(due_date) = &data.due_date {
            let due_date_str = due_date.to_rfc3339();
            guard.execute(
                "UPDATE tasks SET due_date = ?1,
                                  updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                                  synced_at = NULL
                 WHERE id = ?2",
                params![due_date_str, task_id],
            )?;
        }
        if let Some(priority) = &data.priority {
            guard.execute(
                "UPDATE tasks SET priority = ?1,
                                  updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                                  synced_at = NULL
                 WHERE id = ?2",
                params![priority, task_id],
            )?;
        }
        if let Some(repeat_flag) = &data.repeat_flag {
            guard.execute(
                "UPDATE tasks SET repeat_flag = ?1,
                                  updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                                  synced_at = NULL
                 WHERE id = ?2",
                params![repeat_flag, task_id],
            )?;
        }
        Ok(())
    })
    .await?
}

/// Mark a task as completed (status = 2).
pub async fn complete_task(conn: Arc<Mutex<Connection>>, task_id: String) -> Result<()> {
    task::spawn_blocking(move || {
        let guard = conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        guard.execute(
            "UPDATE tasks SET status = 2,
                              updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                              synced_at = NULL
             WHERE id = ?1",
            params![task_id],
        )?;
        Ok(())
    })
    .await?
}

/// Soft-delete a task: marks it as deleted and dirty so the sync loop can
/// propagate the deletion to Postgres before hard-deleting it locally.
pub async fn delete_task(conn: Arc<Mutex<Connection>>, task_id: String) -> Result<()> {
    task::spawn_blocking(move || {
        let guard = conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        guard.execute(
            "UPDATE tasks SET deleted = 1,
                              updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                              synced_at = NULL
             WHERE id = ?1",
            params![task_id],
        )?;
        Ok(())
    })
    .await?
}

/// Hard-delete a task row from SQLite entirely (called after the deletion has
/// been confirmed as synced to Postgres, or when no sync is configured).
pub async fn hard_delete_task(conn: Arc<Mutex<Connection>>, task_id: String) -> Result<()> {
    task::spawn_blocking(move || {
        let guard = conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        guard.execute("DELETE FROM tasks WHERE id = ?1", params![task_id])?;
        Ok(())
    })
    .await?
}

/// Mark a task as synced (set synced_at = now).
pub async fn mark_synced(conn: Arc<Mutex<Connection>>, task_id: String) -> Result<()> {
    task::spawn_blocking(move || {
        let guard = conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        guard.execute(
            "UPDATE tasks SET synced_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE id = ?1",
            params![task_id],
        )?;
        Ok(())
    })
    .await?
}

/// Upsert a task received from the remote (used during pull-sync).
pub async fn upsert_task(conn: Arc<Mutex<Connection>>, task: Task) -> Result<()> {
    task::spawn_blocking(move || {
        let due_date = task.due_date.map(|dt| dt.to_rfc3339());
        let synced_at = task.synced_at.map(|dt| dt.to_rfc3339());
        let updated_at = task.updated_at.to_rfc3339();
        let is_all_day: i64 = if task.is_all_day { 1 } else { 0 };
        let deleted: i64 = if task.deleted { 1 } else { 0 };

        let guard = conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        guard.execute(
            "INSERT INTO tasks (id, project_id, title, content, due_date, priority,
                                repeat_flag, status, is_all_day, updated_at, synced_at, sort_order,
                                deleted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET
                 project_id  = excluded.project_id,
                 title       = excluded.title,
                 content     = excluded.content,
                 due_date    = excluded.due_date,
                 priority    = excluded.priority,
                 repeat_flag = excluded.repeat_flag,
                 status      = excluded.status,
                 is_all_day  = excluded.is_all_day,
                 updated_at  = excluded.updated_at,
                 synced_at   = excluded.synced_at,
                 sort_order  = excluded.sort_order,
                 deleted     = excluded.deleted",
            params![
                task.id,
                task.project_id,
                task.title,
                task.content,
                due_date,
                task.priority,
                task.repeat_flag,
                task.status,
                is_all_day,
                updated_at,
                synced_at,
                task.sort_order,
                deleted
            ],
        )?;
        Ok(())
    })
    .await?
}
