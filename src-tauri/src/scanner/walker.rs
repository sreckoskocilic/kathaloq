use std::collections::HashMap;
use std::path::Path;

use rusqlite::Connection;

use crate::db::{insert_file_entry, update_catalog_stats};
use crate::scanner::media::{extract_and_store_tags, is_media_file};
use crate::scanner::updater::{DiskEntry, walk_disk};

pub fn scan_directory(conn: &Connection, catalog_id: i64, root: &Path) -> Result<(), String> {
    let entries = walk_disk(root)?;
    index_entries(conn, catalog_id, &entries)
}

pub fn index_entries(
    conn: &Connection,
    catalog_id: i64,
    entries: &[DiskEntry],
) -> Result<(), String> {
    let mut path_to_id: HashMap<String, i64> = HashMap::new();
    let mut total_files: u64 = 0;
    let mut total_size: u64 = 0;

    for entry in entries {
        let parent_id = Path::new(&entry.rel_path)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_string_lossy().to_string())
            .and_then(|pp| path_to_id.get(&pp).copied());

        let entry_id = insert_file_entry(
            conn,
            catalog_id,
            parent_id,
            &entry.name,
            &entry.rel_path,
            entry.is_dir,
            entry.size,
            entry.modified.as_deref(),
            entry.extension.as_deref(),
        )
        .map_err(|e| e.to_string())?;

        path_to_id.insert(entry.rel_path.clone(), entry_id);

        if !entry.is_dir {
            total_files += 1;
            total_size += entry.size;

            if is_media_file(entry.extension.as_deref()) {
                extract_and_store_tags(conn, entry_id, &entry.full_path)?;
            }
        }
    }

    update_catalog_stats(conn, catalog_id, total_files, total_size).map_err(|e| e.to_string())?;
    Ok(())
}
