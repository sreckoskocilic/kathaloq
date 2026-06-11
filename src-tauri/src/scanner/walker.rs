use std::collections::HashMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use walkdir::WalkDir;

use crate::db::{insert_file_entry, update_catalog_stats};
use crate::scanner::media::{extract_and_store_tags, is_media_file};
use crate::scanner::should_skip;

pub fn scan_directory(conn: &Connection, catalog_id: i64, root: &Path) -> Result<(), String> {
    let mut path_to_id: HashMap<String, i64> = HashMap::new();
    let mut total_files: u64 = 0;
    let mut total_size: u64 = 0;

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.path() == root {
                return true;
            }
            !should_skip(&e.file_name().to_string_lossy())
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();

        if path == root {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().to_string();
        let rel_path = path.strip_prefix(root).unwrap_or(path).to_string_lossy().to_string();
        let is_dir = metadata.is_dir();
        let size = if is_dir { 0 } else { metadata.len() };

        let modified = metadata.modified().ok().map(|t| {
            let dt: DateTime<Utc> = t.into();
            dt.to_rfc3339()
        });

        let extension = if !is_dir {
            path.extension().map(|e| e.to_string_lossy().to_string())
        } else {
            None
        };

        let parent_path = path
            .parent()
            .and_then(|p| p.strip_prefix(root).ok())
            .map(|p| p.to_string_lossy().to_string());

        let parent_id = parent_path.and_then(|pp| path_to_id.get(&pp).copied());

        let entry_id = match insert_file_entry(
            conn,
            catalog_id,
            parent_id,
            &name,
            &rel_path,
            is_dir,
            size,
            modified.as_deref(),
            extension.as_deref(),
        ) {
            Ok(id) => id,
            Err(_) => continue,
        };

        path_to_id.insert(rel_path, entry_id);

        if !is_dir {
            total_files += 1;
            total_size += size;

            if is_media_file(extension.as_deref()) {
                extract_and_store_tags(conn, entry_id, path)?;
            }
        }
    }

    update_catalog_stats(conn, catalog_id, total_files, total_size).map_err(|e| e.to_string())?;
    Ok(())
}
