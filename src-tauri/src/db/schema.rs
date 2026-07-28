use rusqlite::Connection;

use super::queries::search_key;

const SEARCH_KEY_VERSION: i64 = 1;

pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS catalogs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            root_path TEXT NOT NULL,
            scanned_at TEXT NOT NULL,
            total_files INTEGER NOT NULL DEFAULT 0,
            total_size INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS file_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            catalog_id INTEGER NOT NULL,
            parent_id INTEGER,
            name TEXT NOT NULL,
            name_lower TEXT,
            path TEXT NOT NULL,
            is_dir INTEGER NOT NULL DEFAULT 0,
            size INTEGER NOT NULL DEFAULT 0,
            modified TEXT,
            extension TEXT,
            FOREIGN KEY (catalog_id) REFERENCES catalogs(id) ON DELETE CASCADE,
            FOREIGN KEY (parent_id) REFERENCES file_entries(id)
        );

        CREATE INDEX IF NOT EXISTS idx_entries_catalog ON file_entries(catalog_id);
        -- For raw parent_id joins (descendant walks don't filter by catalog).
        CREATE INDEX IF NOT EXISTS idx_entries_parent ON file_entries(parent_id);
        CREATE INDEX IF NOT EXISTS idx_entries_name ON file_entries(name);
        CREATE INDEX IF NOT EXISTS idx_entries_extension ON file_entries(extension);
        -- get_children: catalog_id + parent_id folder open.
        CREATE INDEX IF NOT EXISTS idx_entries_cat_parent ON file_entries(catalog_id, parent_id);
        -- get_children_filtered matches LOWER(extension); raw index can't serve that.
        CREATE INDEX IF NOT EXISTS idx_entries_cat_ext_lower ON file_entries(catalog_id, LOWER(extension));

        CREATE TABLE IF NOT EXISTS media_tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_entry_id INTEGER NOT NULL UNIQUE,
            duration_secs REAL,
            bitrate INTEGER,
            sample_rate INTEGER,
            channels INTEGER,
            title TEXT,
            artist TEXT,
            album TEXT,
            genre TEXT,
            year INTEGER,
            track_number INTEGER,
            FOREIGN KEY (file_entry_id) REFERENCES file_entries(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_media_tags_entry ON media_tags(file_entry_id);
        ",
    )?;

    backfill_name_lower(conn)?;

    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_entries_name_lower ON file_entries(name_lower);",
    )
}

fn backfill_name_lower(conn: &Connection) -> rusqlite::Result<()> {
    let has_column = conn
        .prepare("SELECT 1 FROM pragma_table_info('file_entries') WHERE name = 'name_lower'")?
        .exists([])?;
    if !has_column {
        conn.execute_batch("ALTER TABLE file_entries ADD COLUMN name_lower TEXT;")?;
    }

    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let sql = if version < SEARCH_KEY_VERSION {
        "SELECT id, name FROM file_entries"
    } else {
        "SELECT id, name FROM file_entries WHERE name_lower IS NULL"
    };

    let stale: Vec<(i64, String)> = conn
        .prepare(sql)?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    if !stale.is_empty() {
        let tx = conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare("UPDATE file_entries SET name_lower = ?1 WHERE id = ?2")?;
            for (id, name) in stale {
                stmt.execute(rusqlite::params![search_key(&name), id])?;
            }
        }
        tx.commit()?;
    }

    if version < SEARCH_KEY_VERSION {
        conn.execute_batch(&format!("PRAGMA user_version = {SEARCH_KEY_VERSION};"))?;
    }
    Ok(())
}
