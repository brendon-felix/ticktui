pub mod local;
pub mod sync;

use anyhow::Result;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Shared handle to the local SQLite database.
/// A Mutex<Connection> is used because rusqlite is synchronous and not Send+Sync by itself.
pub struct Db {
    pub conn: Arc<Mutex<Connection>>,
}

impl Db {
    /// Open (or create) the SQLite database at `path` and run the embedded migration.
    pub fn open(path: &str) -> Result<Arc<Self>> {
        let conn = Connection::open(path)?;

        // Enable WAL for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        // Run the embedded migration SQL
        conn.execute_batch(include_str!("../../migrations/001_initial.sql"))?;

        Ok(Arc::new(Self {
            conn: Arc::new(Mutex::new(conn)),
        }))
    }
}
