use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use super::schema::run_migrations;

#[derive(Clone)]
pub struct Database {
    write_conn: Arc<Mutex<Connection>>,
    read_conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(app_dir: PathBuf) -> Result<Self, rusqlite::Error> {
        std::fs::create_dir_all(&app_dir).ok();
        let db_path = app_dir.join("kathaloq.db");

        let write_conn = Connection::open(&db_path)?;
        write_conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        run_migrations(&write_conn)?;

        // Separate read connection. WAL lets it read the last committed snapshot
        // while the write connection holds a long transaction (e.g. during a
        // scan), so query commands don't block on the writer. query_only guards
        // against accidental writes through this handle.
        let read_conn = Connection::open(&db_path)?;
        read_conn.execute_batch(
            "PRAGMA query_only=ON; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;",
        )?;

        Ok(Self {
            write_conn: Arc::new(Mutex::new(write_conn)),
            read_conn: Arc::new(Mutex::new(read_conn)),
        })
    }

    /// Run a closure against the write connection.
    pub fn with_conn<F, T>(&self, f: F) -> Result<T, rusqlite::Error>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let conn = self.write_conn.lock().unwrap_or_else(|e| e.into_inner());
        f(&conn)
    }

    /// Run a closure against the read-only connection. Does not contend with
    /// the write connection under WAL.
    pub fn with_read_conn<F, T>(&self, f: F) -> Result<T, rusqlite::Error>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let conn = self.read_conn.lock().unwrap_or_else(|e| e.into_inner());
        f(&conn)
    }

    /// Lock the write connection (for transactions needing `&mut Connection`).
    pub fn lock(&self) -> MutexGuard<'_, Connection> {
        self.write_conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}
