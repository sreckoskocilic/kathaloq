use rusqlite::Connection;
use rusqlite::ffi::ErrorCode;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use super::schema::run_migrations;

fn is_corruption(error: &rusqlite::Error) -> bool {
    matches!(
        error.sqlite_error_code(),
        Some(ErrorCode::DatabaseCorrupt) | Some(ErrorCode::NotADatabase)
    )
}

#[derive(Clone)]
pub struct Database {
    write_conn: Arc<Mutex<Connection>>,
    read_conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(app_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&app_dir)
            .map_err(|e| format!("Cannot create {}: {e}", app_dir.display()))?;
        let db_path = app_dir.join("kathaloq.db");

        match Self::open_at(&db_path) {
            Ok(db) => Ok(db),
            Err(e) if !is_corruption(&e) => Err(format!(
                "Cannot open {}: {e}. The existing database was left untouched.",
                db_path.display()
            )),
            Err(corrupt) => {
                let quarantine = db_path.with_extension("db.corrupt");
                std::fs::rename(&db_path, &quarantine).map_err(|e| {
                    format!(
                        "Database at {} is corrupt ({corrupt}), and it could not be moved aside: {e}",
                        db_path.display()
                    )
                })?;
                Self::open_at(&db_path).map_err(|e| {
                    format!(
                        "Could not recreate the database after quarantining {}: {e}",
                        quarantine.display()
                    )
                })
            }
        }
    }

    fn open_at(db_path: &std::path::Path) -> Result<Self, rusqlite::Error> {
        let write_conn = Connection::open(db_path)?;
        write_conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;",
        )?;
        run_migrations(&write_conn)?;

        let read_conn = Connection::open(db_path)?;
        read_conn.execute_batch(
            "PRAGMA query_only=ON; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;",
        )?;

        Ok(Self {
            write_conn: Arc::new(Mutex::new(write_conn)),
            read_conn: Arc::new(Mutex::new(read_conn)),
        })
    }

    pub fn with_conn<F, T>(&self, f: F) -> Result<T, rusqlite::Error>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let conn = self.write_conn.lock().unwrap_or_else(|e| e.into_inner());
        f(&conn)
    }

    pub fn with_read_conn<F, T>(&self, f: F) -> Result<T, rusqlite::Error>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let conn = self.read_conn.lock().unwrap_or_else(|e| e.into_inner());
        f(&conn)
    }

    pub fn lock(&self) -> MutexGuard<'_, Connection> {
        self.write_conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn read_lock(&self) -> MutexGuard<'_, Connection> {
        self.read_conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("kathaloq-conn-{tag}-{nanos}"));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn a_corrupt_file_is_quarantined_and_replaced() {
        let dir = temp_dir("corrupt");
        let db_path = dir.join("kathaloq.db");
        std::fs::write(&db_path, b"definitely not a sqlite file").unwrap();

        let db = Database::new(dir.clone()).expect("must recover");
        assert!(
            dir.join("kathaloq.db.corrupt").exists(),
            "the bad file must be kept for inspection"
        );
        db.with_read_conn(|c| {
            c.query_row("SELECT COUNT(*) FROM catalogs", [], |r| r.get::<_, i64>(0))
        })
        .expect("a usable database must exist");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_non_corruption_failure_leaves_the_database_alone() {
        let dir = temp_dir("locked");
        let db_path = dir.join("kathaloq.db");
        std::fs::create_dir(&db_path).unwrap();

        let err = match Database::new(dir.clone()) {
            Err(e) => e,
            Ok(_) => panic!("opening a directory as the database must fail"),
        };
        assert!(err.contains("left untouched"), "got: {err}");
        assert!(db_path.is_dir(), "the existing file must not be renamed");
        assert!(!dir.join("kathaloq.db.corrupt").exists());

        std::fs::remove_dir_all(&dir).ok();
    }
}
