use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;
use tokio_postgres::Client as PgClient;

use crate::{app::AppAction, debug};

use super::local;

/// Push all dirty local tasks to Postgres, then pull any rows from Postgres
/// that are newer than our most-recent synced_at timestamp.
///
/// Returns `true` if any rows were exchanged (so the caller knows to refresh).
pub async fn sync_once(conn: Arc<Mutex<Connection>>, pg: &PgClient) -> Result<bool> {
    let mut changed = false;

    // --- Push dirty rows up ---
    let dirty = local::fetch_dirty_tasks(Arc::clone(&conn)).await?;
    for task in &dirty {
        let due_date_str = task.due_date.map(|dt| dt.to_rfc3339());
        let due_date: Option<&str> = due_date_str.as_deref();
        let updated_at = task.updated_at.to_rfc3339();
        let is_all_day: i32 = if task.is_all_day { 1 } else { 0 };
        let priority: i32 = task.priority as i32;
        let status: i32 = task.status as i32;
        let sort_order: i32 = task.sort_order as i32;

        pg.execute(
            "INSERT INTO tasks (id, project_id, title, content, due_date, priority,
                                repeat_flag, status, is_all_day, updated_at, sort_order)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
             ON CONFLICT (id) DO UPDATE SET
                 project_id  = EXCLUDED.project_id,
                 title       = EXCLUDED.title,
                 content     = EXCLUDED.content,
                 due_date    = EXCLUDED.due_date,
                 priority    = EXCLUDED.priority,
                 repeat_flag = EXCLUDED.repeat_flag,
                 status      = EXCLUDED.status,
                 is_all_day  = EXCLUDED.is_all_day,
                 updated_at  = EXCLUDED.updated_at,
                 sort_order  = EXCLUDED.sort_order",
            &[
                &task.id.as_str(),
                &task.project_id.as_str(),
                &task.title.as_str(),
                &task.content.as_str(),
                &due_date,
                &priority,
                &task.repeat_flag.as_str(),
                &status,
                &is_all_day,
                &updated_at.as_str(),
                &sort_order,
            ],
        )
        .await?;

        local::mark_synced(Arc::clone(&conn), task.id.clone()).await?;
        changed = true;
    }

    // --- Pull rows that are newer than our last known sync watermark ---
    let last_synced: Option<DateTime<Utc>> = {
        let guard = conn
            .lock()
            .map_err(|e| anyhow::anyhow!("mutex poisoned: {e}"))?;
        guard
            .query_row("SELECT MAX(synced_at) FROM tasks", [], |row| {
                row.get::<_, Option<String>>(0)
            })
            .unwrap_or(None)
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
    };

    let watermark = last_synced
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    let rows = pg
        .query(
            "SELECT id, project_id, title, content, due_date, priority, repeat_flag,
                    status, is_all_day, updated_at, sort_order
             FROM tasks
             WHERE updated_at > $1",
            &[&watermark],
        )
        .await?;

    for row in &rows {
        let due_date_str: Option<String> = row.get(4);
        let due_date = due_date_str
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let updated_at_str: String = row.get(9);
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .unwrap_or_default()
            .with_timezone(&Utc);

        let is_all_day_int: i32 = row.get(8);

        let task = crate::tasks::Task {
            id: row.get(0),
            project_id: row.get(1),
            title: row.get(2),
            content: row.get(3),
            due_date,
            priority: row.get::<_, i32>(5) as i64,
            repeat_flag: row.get(6),
            status: row.get::<_, i32>(7) as i64,
            is_all_day: is_all_day_int != 0,
            sort_order: row.get::<_, i32>(10) as i64,
            updated_at,
            synced_at: Some(Utc::now()),
        };
        local::upsert_task(Arc::clone(&conn), task).await?;
        changed = true;
    }

    Ok(changed)
}

/// Spawn a background task that syncs every 30 seconds.
/// `pg_url` is a standard libpq connection string, e.g. "host=localhost dbname=ticktui".
pub async fn run_sync_loop(
    conn: Arc<Mutex<Connection>>,
    pg_url: String,
    tx: UnboundedSender<AppAction>,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
    loop {
        interval.tick().await;

        // Re-connect on every iteration so transient failures self-heal
        let pg_result = tokio_postgres::connect(&pg_url, tokio_postgres::NoTls).await;
        match pg_result {
            Err(e) => {
                debug!("Postgres connect failed: {:#}", e);
            }
            Ok((pg_client, pg_conn)) => {
                // Drive the connection on a separate task
                tokio::spawn(async move {
                    if let Err(_e) = pg_conn.await {
                        // Connection dropped; next loop iteration will reconnect
                    }
                });

                match sync_once(Arc::clone(&conn), &pg_client).await {
                    Ok(true) => {
                        let _ = tx.send(AppAction::RefreshData);
                    }
                    Ok(false) => {}
                    Err(e) => {
                        debug!("Sync error: {:#}", e);
                    }
                }
            }
        }
    }
}
