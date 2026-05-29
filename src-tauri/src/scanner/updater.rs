use std::collections::HashMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use walkdir::WalkDir;

use std::collections::HashSet;

use crate::db::{
    delete_file_entries_by_ids, get_all_entries, get_media_tags, insert_file_entry,
    update_catalog_scanned_at, update_catalog_stats, update_file_entry_metadata,
};
use crate::models::{FileEntry, UpdatePreview};
use crate::scanner::media::{extract_and_store_tags, is_media_file};
use crate::scanner::should_skip;

struct DiskEntry {
    name: String,
    rel_path: String,
    full_path: std::path::PathBuf,
    is_dir: bool,
    size: u64,
    modified: Option<String>,
    extension: Option<String>,
}

fn walk_disk(root: &Path) -> Vec<DiskEntry> {
    let mut entries = Vec::new();

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
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
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

        entries.push(DiskEntry {
            name,
            rel_path,
            full_path: path.to_path_buf(),
            is_dir,
            size,
            modified,
            extension,
        });
    }

    entries
}

pub fn preview_update(conn: &Connection, catalog_id: i64, root: &Path) -> Result<UpdatePreview, String> {
    let db_entries = get_all_entries(conn, catalog_id).map_err(|e| e.to_string())?;
    let mut db_map: HashMap<String, &FileEntry> = HashMap::new();
    for entry in &db_entries {
        db_map.insert(entry.path.clone(), entry);
    }

    let disk_entries = walk_disk(root);

    let mut added: u64 = 0;
    let mut updated: u64 = 0;
    let mut unchanged: u64 = 0;
    let mut tags_to_backfill: u64 = 0;
    let mut seen_paths: HashSet<String> = HashSet::new();

    for disk in &disk_entries {
        seen_paths.insert(disk.rel_path.clone());
        match db_map.get(&disk.rel_path) {
            Some(db_entry) => {
                if !disk.is_dir && (db_entry.size != disk.size || db_entry.modified != disk.modified) {
                    updated += 1;
                } else {
                    if !disk.is_dir && is_media_file(disk.extension.as_deref()) {
                        let has_tags = get_media_tags(conn, db_entry.id)
                            .map(|t| t.is_some())
                            .unwrap_or(false);
                        if !has_tags {
                            tags_to_backfill += 1;
                        }
                    }
                    unchanged += 1;
                }
            }
            None => {
                added += 1;
                if !disk.is_dir && is_media_file(disk.extension.as_deref()) {
                    tags_to_backfill += 1;
                }
            }
        }
    }

    let mut deleted_files: u64 = 0;
    let mut deleted_folders: u64 = 0;
    for entry in &db_entries {
        if !seen_paths.contains(&entry.path) {
            if entry.is_dir {
                deleted_folders += 1;
            } else {
                deleted_files += 1;
            }
        }
    }

    Ok(UpdatePreview {
        added,
        updated,
        deleted_files,
        deleted_folders,
        unchanged,
        tags_to_backfill,
    })
}

pub fn apply_update(conn: &Connection, catalog_id: i64, root: &Path) -> Result<UpdatePreview, String> {
    let db_entries = get_all_entries(conn, catalog_id).map_err(|e| e.to_string())?;
    let mut db_map: HashMap<String, FileEntry> = HashMap::new();
    for entry in db_entries {
        db_map.insert(entry.path.clone(), entry);
    }

    let disk_entries = walk_disk(root);

    let mut added: u64 = 0;
    let mut updated: u64 = 0;
    let mut unchanged: u64 = 0;
    let mut tags_backfilled: u64 = 0;
    let mut seen_paths: HashSet<String> = HashSet::new();

    let mut path_to_id: HashMap<String, i64> = HashMap::new();
    for (path, entry) in &db_map {
        path_to_id.insert(path.clone(), entry.id);
    }

    for disk in &disk_entries {
        seen_paths.insert(disk.rel_path.clone());

        let parent_path = Path::new(&disk.rel_path)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_string_lossy().to_string());
        let parent_id = parent_path.and_then(|pp| path_to_id.get(&pp).copied());

        match db_map.get(&disk.rel_path) {
            Some(db_entry) => {
                path_to_id.insert(disk.rel_path.clone(), db_entry.id);
                if !disk.is_dir && (db_entry.size != disk.size || db_entry.modified != disk.modified) {
                    update_file_entry_metadata(
                        conn,
                        db_entry.id,
                        disk.size,
                        disk.modified.as_deref(),
                    )
                    .map_err(|e| e.to_string())?;
                    if is_media_file(disk.extension.as_deref())
                        && extract_and_store_tags(conn, db_entry.id, &disk.full_path)
                    {
                        tags_backfilled += 1;
                    }
                    updated += 1;
                } else {
                    if !disk.is_dir && is_media_file(disk.extension.as_deref()) {
                        let has_tags = get_media_tags(conn, db_entry.id)
                            .map(|t| t.is_some())
                            .unwrap_or(false);
                        if !has_tags
                            && extract_and_store_tags(conn, db_entry.id, &disk.full_path)
                        {
                            tags_backfilled += 1;
                        }
                    }
                    unchanged += 1;
                }
            }
            None => {
                let entry_id = insert_file_entry(
                    conn,
                    catalog_id,
                    parent_id,
                    &disk.name,
                    &disk.rel_path,
                    disk.is_dir,
                    disk.size,
                    disk.modified.as_deref(),
                    disk.extension.as_deref(),
                )
                .map_err(|e| e.to_string())?;
                path_to_id.insert(disk.rel_path.clone(), entry_id);
                if !disk.is_dir && is_media_file(disk.extension.as_deref())
                    && extract_and_store_tags(conn, entry_id, &disk.full_path)
                {
                    tags_backfilled += 1;
                }
                added += 1;
            }
        }
    }

    let mut deleted_files: u64 = 0;
    let mut deleted_folders: u64 = 0;
    let mut ids_to_delete: Vec<i64> = Vec::new();

    for (path, entry) in &db_map {
        if !seen_paths.contains(path) {
            if entry.is_dir {
                deleted_folders += 1;
            } else {
                deleted_files += 1;
            }
            ids_to_delete.push(entry.id);
        }
    }

    if !ids_to_delete.is_empty() {
        delete_file_entries_by_ids(conn, &ids_to_delete).map_err(|e| e.to_string())?;
    }

    let total_files = (unchanged + updated + added) - disk_entries.iter().filter(|e| e.is_dir).count() as u64;
    let total_size: u64 = disk_entries.iter().filter(|e| !e.is_dir).map(|e| e.size).sum();
    update_catalog_stats(conn, catalog_id, total_files, total_size).map_err(|e| e.to_string())?;

    let scanned_at = Utc::now().to_rfc3339();
    update_catalog_scanned_at(conn, catalog_id, &scanned_at).map_err(|e| e.to_string())?;

    Ok(UpdatePreview {
        added,
        updated,
        deleted_files,
        deleted_folders,
        unchanged,
        tags_to_backfill: tags_backfilled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{get_all_entries, get_catalog_by_id, insert_catalog, run_migrations};
    use crate::scanner::scan_directory;
    use rusqlite::Connection;
    use std::path::PathBuf;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("kathaloq-test-{tag}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            TempDir { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run_migrations(&c).unwrap();
        c
    }

    fn write(root: &Path, rel: &str, content: &[u8]) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn scan_directory_indexes_tree_and_skips_hidden() {
        let tmp = TempDir::new("scan");
        write(&tmp.path, "root.txt", b"hello");
        write(&tmp.path, "sub/a.txt", b"ab");
        write(&tmp.path, ".hidden", b"x");

        let c = conn();
        let cat = insert_catalog(&c, "t", "/r", "2026-01-01T00:00:00Z").unwrap();
        scan_directory(&c, cat, &tmp.path).unwrap();

        let entries = get_all_entries(&c, cat).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(entries.len(), 3, "root.txt, sub, sub/a.txt; .hidden skipped");
        assert!(!names.contains(&".hidden"));

        let sub = entries.iter().find(|e| e.name == "sub").unwrap();
        let a = entries.iter().find(|e| e.name == "a.txt").unwrap();
        assert_eq!(a.parent_id, Some(sub.id), "a.txt parented under sub");

        let cat_row = get_catalog_by_id(&c, cat).unwrap();
        assert_eq!(cat_row.total_files, 2);
        assert_eq!(cat_row.total_size, 7);
    }

    #[test]
    fn apply_update_reconciles_add_update_delete() {
        let tmp = TempDir::new("apply");
        write(&tmp.path, "keep.txt", b"AAAA");
        write(&tmp.path, "change.txt", b"AAAA");
        write(&tmp.path, "sub/del.txt", b"X");

        let c = conn();
        let cat = insert_catalog(&c, "t", "/r", "2026-01-01T00:00:00Z").unwrap();
        scan_directory(&c, cat, &tmp.path).unwrap();
        assert_eq!(get_all_entries(&c, cat).unwrap().len(), 4);

        // Mutate the tree: grow one file, delete one, add one.
        write(&tmp.path, "change.txt", b"AAAAAAAA");
        std::fs::remove_file(tmp.path.join("sub/del.txt")).unwrap();
        write(&tmp.path, "new.txt", b"NN");

        let preview = apply_update(&c, cat, &tmp.path).unwrap();
        assert_eq!(preview.added, 1, "new.txt");
        assert_eq!(preview.updated, 1, "change.txt");
        assert_eq!(preview.deleted_files, 1, "del.txt");
        assert_eq!(preview.deleted_folders, 0);
        assert_eq!(preview.unchanged, 2, "keep.txt + sub dir");

        let entries = get_all_entries(&c, cat).unwrap();
        assert_eq!(entries.len(), 4, "keep, change, sub, new");
        assert!(entries.iter().all(|e| e.name != "del.txt"));

        let cat_row = get_catalog_by_id(&c, cat).unwrap();
        assert_eq!(cat_row.total_files, 3, "keep, change, new");
    }

    #[test]
    fn preview_update_does_not_mutate_db() {
        let tmp = TempDir::new("preview");
        write(&tmp.path, "keep.txt", b"AAAA");
        write(&tmp.path, "sub/del.txt", b"X");

        let c = conn();
        let cat = insert_catalog(&c, "t", "/r", "2026-01-01T00:00:00Z").unwrap();
        scan_directory(&c, cat, &tmp.path).unwrap();
        let before = get_all_entries(&c, cat).unwrap().len();

        write(&tmp.path, "new.txt", b"NN");
        let preview = preview_update(&c, cat, &tmp.path).unwrap();
        assert_eq!(preview.added, 1);
        assert_eq!(preview.deleted_files, 0);

        // DB untouched by a preview.
        assert_eq!(get_all_entries(&c, cat).unwrap().len(), before);
    }
}
