use std::collections::HashMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use walkdir::WalkDir;

use std::collections::HashSet;

use crate::db::{
    collect_descendant_ids, delete_file_entries_by_ids, get_all_entries, get_media_tags,
    insert_file_entry, recalc_catalog_stats, update_catalog_scanned_at, update_file_entry_metadata,
};
use crate::models::{FileEntry, UpdatePreview};
use crate::scanner::media::{extract_and_store_tags, is_media_file};
use crate::scanner::should_skip;

pub struct DiskEntry {
    pub name: String,
    pub rel_path: String,
    pub full_path: std::path::PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<String>,
    pub extension: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WalkPolicy {
    Strict,
    SkipUnreadable,
}

pub fn walk_disk(root: &Path, policy: WalkPolicy) -> Result<Vec<DiskEntry>, String> {
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
            Err(e) if policy == WalkPolicy::SkipUnreadable => {
                let _ = e;
                continue;
            }
            Err(e) => return Err(format!("Cannot read directory during scan: {e}")),
        };
        let path = entry.path();

        if path == root {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(e) if policy == WalkPolicy::SkipUnreadable => {
                let _ = e;
                continue;
            }
            Err(e) => {
                return Err(format!("Cannot read metadata for {}: {e}", path.display()));
            }
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

    Ok(entries)
}

// Tree depth of a stored relative path, used to delete children before parents.
fn path_depth(path: &str) -> usize {
    path.chars().filter(|&c| c == '/' || c == '\\').count()
}

fn guard_vanished_root(disk_entries: &[DiskEntry], db_len: usize) -> Result<(), String> {
    if disk_entries.is_empty() && db_len > 0 {
        return Err(
            "Root path is empty — is the drive mounted? Refusing to wipe the catalog.".to_string(),
        );
    }
    Ok(())
}

fn flipped_ids(db_map: &HashMap<String, FileEntry>, disk_entries: &[DiskEntry]) -> Vec<i64> {
    disk_entries
        .iter()
        .filter_map(|d| db_map.get(&d.rel_path).filter(|e| e.is_dir != d.is_dir))
        .map(|e| e.id)
        .collect()
}

fn collect_type_flipped(
    conn: &Connection,
    catalog_id: i64,
    db_map: &HashMap<String, FileEntry>,
    disk_entries: &[DiskEntry],
) -> Result<Vec<i64>, String> {
    let roots = flipped_ids(db_map, disk_entries);
    if roots.is_empty() {
        return Ok(Vec::new());
    }
    collect_descendant_ids(conn, catalog_id, &roots).map_err(|e| e.to_string())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Preview,
    Apply,
}

fn is_media_entry(disk: &DiskEntry) -> bool {
    !disk.is_dir && is_media_file(disk.extension.as_deref())
}

fn backfill_tags(
    conn: &Connection,
    mode: Mode,
    disk: &DiskEntry,
    entry_id: i64,
) -> Result<bool, String> {
    if !is_media_entry(disk) {
        return Ok(false);
    }
    match mode {
        Mode::Preview => Ok(true),
        Mode::Apply => extract_and_store_tags(conn, entry_id, &disk.full_path),
    }
}

fn drop_type_flipped(
    conn: &Connection,
    catalog_id: i64,
    db_map: &mut HashMap<String, FileEntry>,
    disk_entries: &[DiskEntry],
    mode: Mode,
) -> Result<(u64, u64), String> {
    let stale = collect_type_flipped(conn, catalog_id, db_map, disk_entries)?;
    if stale.is_empty() {
        return Ok((0, 0));
    }
    if mode == Mode::Apply {
        delete_file_entries_by_ids(conn, catalog_id, &stale).map_err(|e| e.to_string())?;
    }

    let stale: HashSet<i64> = stale.into_iter().collect();
    let mut files: u64 = 0;
    let mut folders: u64 = 0;
    for entry in db_map.values().filter(|e| stale.contains(&e.id)) {
        if entry.is_dir {
            folders += 1;
        } else {
            files += 1;
        }
    }
    db_map.retain(|_, e| !stale.contains(&e.id));
    Ok((files, folders))
}

enum Synced {
    Updated { tagged: bool },
    Unchanged { tagged: bool },
}

fn sync_existing(
    conn: &Connection,
    mode: Mode,
    disk: &DiskEntry,
    db_entry: &FileEntry,
) -> Result<Synced, String> {
    if db_entry.size != disk.size || db_entry.modified != disk.modified {
        if mode == Mode::Apply {
            update_file_entry_metadata(conn, db_entry.id, disk.size, disk.modified.as_deref())
                .map_err(|e| e.to_string())?;
        }
        return Ok(Synced::Updated {
            tagged: backfill_tags(conn, mode, disk, db_entry.id)?,
        });
    }

    if !is_media_entry(disk) {
        return Ok(Synced::Unchanged { tagged: false });
    }
    let has_tags = get_media_tags(conn, db_entry.id)
        .map(|t| t.is_some())
        .unwrap_or(false);
    Ok(Synced::Unchanged {
        tagged: !has_tags && backfill_tags(conn, mode, disk, db_entry.id)?,
    })
}

fn insert_new(
    conn: &Connection,
    catalog_id: i64,
    mode: Mode,
    disk: &DiskEntry,
    parent_id: Option<i64>,
    path_to_id: &mut HashMap<String, i64>,
) -> Result<bool, String> {
    if mode == Mode::Preview {
        return Ok(is_media_entry(disk));
    }

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
    backfill_tags(conn, mode, disk, entry_id)
}

pub fn preview_update(
    conn: &Connection,
    catalog_id: i64,
    root: &Path,
) -> Result<UpdatePreview, String> {
    let disk_entries = walk_disk(root, WalkPolicy::Strict)?;
    preview_update_entries(conn, catalog_id, &disk_entries)
}

pub fn preview_update_entries(
    conn: &Connection,
    catalog_id: i64,
    disk_entries: &[DiskEntry],
) -> Result<UpdatePreview, String> {
    reconcile(conn, catalog_id, disk_entries, Mode::Preview)
}

pub fn apply_update(
    conn: &Connection,
    catalog_id: i64,
    root: &Path,
) -> Result<UpdatePreview, String> {
    let disk_entries = walk_disk(root, WalkPolicy::Strict)?;
    apply_update_entries(conn, catalog_id, &disk_entries)
}

pub fn apply_update_entries(
    conn: &Connection,
    catalog_id: i64,
    disk_entries: &[DiskEntry],
) -> Result<UpdatePreview, String> {
    reconcile(conn, catalog_id, disk_entries, Mode::Apply)
}

fn reconcile(
    conn: &Connection,
    catalog_id: i64,
    disk_entries: &[DiskEntry],
    mode: Mode,
) -> Result<UpdatePreview, String> {
    let db_entries = get_all_entries(conn, catalog_id).map_err(|e| e.to_string())?;
    let mut db_map: HashMap<String, FileEntry> = HashMap::new();
    for entry in db_entries {
        db_map.insert(entry.path.clone(), entry);
    }

    guard_vanished_root(disk_entries, db_map.len())?;

    let mut added: u64 = 0;
    let mut updated: u64 = 0;
    let mut unchanged: u64 = 0;
    let mut tags_to_backfill: u64 = 0;
    let mut deleted_files: u64 = 0;
    let mut deleted_folders: u64 = 0;
    let mut seen_paths: HashSet<String> = HashSet::new();

    let (flipped_files, flipped_folders) =
        drop_type_flipped(conn, catalog_id, &mut db_map, disk_entries, mode)?;
    deleted_files += flipped_files;
    deleted_folders += flipped_folders;

    let mut path_to_id: HashMap<String, i64> = HashMap::new();
    for (path, entry) in &db_map {
        path_to_id.insert(path.clone(), entry.id);
    }

    for disk in disk_entries {
        seen_paths.insert(disk.rel_path.clone());

        let parent_path = Path::new(&disk.rel_path)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_string_lossy().to_string());
        let parent_id = parent_path.and_then(|pp| path_to_id.get(&pp).copied());

        match db_map.get(&disk.rel_path) {
            Some(db_entry) => {
                path_to_id.insert(disk.rel_path.clone(), db_entry.id);
                let tagged = match sync_existing(conn, mode, disk, db_entry)? {
                    Synced::Updated { tagged } => {
                        updated += 1;
                        tagged
                    }
                    Synced::Unchanged { tagged } => {
                        unchanged += 1;
                        tagged
                    }
                };
                if tagged {
                    tags_to_backfill += 1;
                }
            }
            None => {
                if insert_new(conn, catalog_id, mode, disk, parent_id, &mut path_to_id)? {
                    tags_to_backfill += 1;
                }
                added += 1;
            }
        }
    }

    let mut to_delete: Vec<&FileEntry> = Vec::new();

    for (path, entry) in &db_map {
        if !seen_paths.contains(path) {
            if entry.is_dir {
                deleted_folders += 1;
            } else {
                deleted_files += 1;
            }
            to_delete.push(entry);
        }
    }

    // Sort parents before children; delete removes in reverse (children first),
    // so the parent_id FK holds without defer_foreign_keys.
    to_delete.sort_by_key(|e| path_depth(&e.path));
    let ids_to_delete: Vec<i64> = to_delete.iter().map(|e| e.id).collect();

    if mode == Mode::Apply && !ids_to_delete.is_empty() {
        delete_file_entries_by_ids(conn, catalog_id, &ids_to_delete).map_err(|e| e.to_string())?;
    }

    // Totals from the rows, not the disk-walk tally, so a mid-scan hiccup can't
    // desync catalogs.total_*.
    if mode == Mode::Apply {
        recalc_catalog_stats(conn, catalog_id).map_err(|e| e.to_string())?;

        let scanned_at = Utc::now().to_rfc3339();
        update_catalog_scanned_at(conn, catalog_id, &scanned_at).map_err(|e| e.to_string())?;
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
            let path = std::env::temp_dir().join(format!(
                "kathaloq-test-{tag}-{}-{nanos}",
                std::process::id()
            ));
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
        assert_eq!(
            entries.len(),
            3,
            "root.txt, sub, sub/a.txt; .hidden skipped"
        );
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
        assert_eq!(preview.updated, 2, "change.txt + sub, whose mtime moved");
        assert_eq!(preview.deleted_files, 1, "del.txt");
        assert_eq!(preview.deleted_folders, 0);
        assert_eq!(preview.unchanged, 1, "keep.txt");

        let entries = get_all_entries(&c, cat).unwrap();
        assert_eq!(entries.len(), 4, "keep, change, sub, new");
        assert!(entries.iter().all(|e| e.name != "del.txt"));

        let cat_row = get_catalog_by_id(&c, cat).unwrap();
        assert_eq!(cat_row.total_files, 3, "keep, change, new");
    }

    #[test]
    fn apply_update_deletes_nested_subtree_without_fk_violation() {
        // Whole subtree vanishes: children must delete before parents (parent_id FK).
        let tmp = TempDir::new("subtree");
        write(&tmp.path, "root.txt", b"R");
        write(&tmp.path, "a/b/c.txt", b"C");
        write(&tmp.path, "a/b/d.txt", b"D");

        let c = conn();
        let cat = insert_catalog(&c, "t", "/r", "2026-01-01T00:00:00Z").unwrap();
        scan_directory(&c, cat, &tmp.path).unwrap();
        assert_eq!(
            get_all_entries(&c, cat).unwrap().len(),
            5,
            "root.txt, a, a/b, a/b/c.txt, a/b/d.txt"
        );

        // Remove the entire `a` subtree, keep the root sibling.
        std::fs::remove_dir_all(tmp.path.join("a")).unwrap();

        let preview = apply_update(&c, cat, &tmp.path).unwrap();
        assert_eq!(preview.deleted_folders, 2, "a + a/b");
        assert_eq!(preview.deleted_files, 2, "c.txt + d.txt");
        assert_eq!(preview.unchanged, 1, "root.txt");
        assert_eq!(preview.added, 0);

        let entries = get_all_entries(&c, cat).unwrap();
        assert_eq!(entries.len(), 1, "only root.txt survives");
        assert_eq!(entries[0].name, "root.txt");

        let cat_row = get_catalog_by_id(&c, cat).unwrap();
        assert_eq!(cat_row.total_files, 1);
    }

    #[test]
    fn apply_update_handles_dir_rename_as_delete_add_with_reparent() {
        // Dir rename = delete old tree + add new; child must re-parent under the new dir.
        let tmp = TempDir::new("rename");
        write(&tmp.path, "old/file.txt", b"X");

        let c = conn();
        let cat = insert_catalog(&c, "t", "/r", "2026-01-01T00:00:00Z").unwrap();
        scan_directory(&c, cat, &tmp.path).unwrap();
        assert_eq!(
            get_all_entries(&c, cat).unwrap().len(),
            2,
            "old, old/file.txt"
        );

        std::fs::rename(tmp.path.join("old"), tmp.path.join("new")).unwrap();

        let preview = apply_update(&c, cat, &tmp.path).unwrap();
        assert_eq!(preview.added, 2, "new dir + new/file.txt");
        assert_eq!(preview.deleted_folders, 1, "old");
        assert_eq!(preview.deleted_files, 1, "old/file.txt");

        let entries = get_all_entries(&c, cat).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(
            entries.iter().all(|e| !e.path.starts_with("old")),
            "old path gone"
        );
        let new_dir = entries.iter().find(|e| e.name == "new").unwrap();
        let child = entries.iter().find(|e| e.name == "file.txt").unwrap();
        assert_eq!(
            child.parent_id,
            Some(new_dir.id),
            "child re-parented under new dir"
        );
    }

    #[test]
    fn apply_update_is_idempotent_on_unchanged_tree() {
        // A re-scan with no disk changes must produce zero churn and stable rows/stats.
        let tmp = TempDir::new("idempotent");
        write(&tmp.path, "root.txt", b"R");
        write(&tmp.path, "sub/a.txt", b"AB");

        let c = conn();
        let cat = insert_catalog(&c, "t", "/r", "2026-01-01T00:00:00Z").unwrap();
        scan_directory(&c, cat, &tmp.path).unwrap();
        let before = get_all_entries(&c, cat).unwrap().len();
        let stats_before = get_catalog_by_id(&c, cat).unwrap();

        let preview = apply_update(&c, cat, &tmp.path).unwrap();
        assert_eq!(preview.added, 0);
        assert_eq!(preview.updated, 0);
        assert_eq!(preview.deleted_files, 0);
        assert_eq!(preview.deleted_folders, 0);
        assert_eq!(preview.unchanged, before as u64, "every entry unchanged");

        let stats_after = get_catalog_by_id(&c, cat).unwrap();
        assert_eq!(
            get_all_entries(&c, cat).unwrap().len(),
            before,
            "row count stable"
        );
        assert_eq!(stats_after.total_files, stats_before.total_files);
        assert_eq!(stats_after.total_size, stats_before.total_size);
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

        assert_eq!(get_all_entries(&c, cat).unwrap().len(), before);
    }

    #[test]
    fn directory_mtime_changes_are_written_back() {
        let tmp = TempDir::new("dirmtime");
        write(&tmp.path, "sub/one.txt", b"A");

        let c = conn();
        let cat = insert_catalog(&c, "t", "/r", "2026-01-01T00:00:00Z").unwrap();
        scan_directory(&c, cat, &tmp.path).unwrap();
        let before = get_all_entries(&c, cat)
            .unwrap()
            .into_iter()
            .find(|e| e.name == "sub")
            .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(1100));
        write(&tmp.path, "sub/two.txt", b"B");
        apply_update(&c, cat, &tmp.path).unwrap();

        let after = get_all_entries(&c, cat)
            .unwrap()
            .into_iter()
            .find(|e| e.name == "sub")
            .unwrap();
        assert_ne!(
            before.modified, after.modified,
            "a folder touched after import must not keep its first-scan timestamp"
        );
        assert_eq!(after.size, 0, "folders stay sizeless");
    }

    #[test]
    fn initial_scan_indexes_what_it_can_reach_past_an_unreadable_dir() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new("perm");
        write(&tmp.path, "readable/a.txt", b"A");
        let locked = tmp.path.join("locked");
        std::fs::create_dir_all(locked.join("inner")).unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let lenient = walk_disk(&tmp.path, WalkPolicy::SkipUnreadable);
        let strict = walk_disk(&tmp.path, WalkPolicy::Strict);

        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

        let lenient = lenient.expect("a first import must survive one unreadable folder");
        assert!(
            lenient.iter().any(|e| e.name == "a.txt"),
            "reachable files must still be indexed"
        );
        assert!(
            strict.is_err(),
            "reconcile must still refuse a partial walk"
        );
    }

    #[test]
    fn empty_root_refuses_instead_of_wiping_the_catalog() {
        let tmp = TempDir::new("unmounted");
        write(&tmp.path, "music/a.mp3", b"x");
        write(&tmp.path, "music/b.mp3", b"y");

        let c = conn();
        let cat = insert_catalog(&c, "t", "/r", "2026-01-01T00:00:00Z").unwrap();
        scan_directory(&c, cat, &tmp.path).unwrap();
        let before = get_all_entries(&c, cat).unwrap().len();
        assert!(before > 0);

        std::fs::remove_dir_all(tmp.path.join("music")).unwrap();

        let err = preview_update(&c, cat, &tmp.path).unwrap_err();
        assert!(err.contains("drive mounted"), "got: {err}");
        let err = apply_update(&c, cat, &tmp.path).unwrap_err();
        assert!(err.contains("drive mounted"), "got: {err}");

        assert_eq!(
            get_all_entries(&c, cat).unwrap().len(),
            before,
            "an empty root must not delete a single row"
        );
    }

    #[test]
    fn file_replaced_by_directory_is_reindexed_with_the_right_type() {
        let tmp = TempDir::new("flip");
        write(&tmp.path, "Live", b"was a file");

        let c = conn();
        let cat = insert_catalog(&c, "t", "/r", "2026-01-01T00:00:00Z").unwrap();
        scan_directory(&c, cat, &tmp.path).unwrap();
        assert!(!get_all_entries(&c, cat).unwrap()[0].is_dir);

        std::fs::remove_file(tmp.path.join("Live")).unwrap();
        write(&tmp.path, "Live/track.mp3", b"audio");

        apply_update(&c, cat, &tmp.path).unwrap();

        let entries = get_all_entries(&c, cat).unwrap();
        let live = entries.iter().find(|e| e.path == "Live").unwrap();
        let track = entries.iter().find(|e| e.name == "track.mp3").unwrap();
        assert!(live.is_dir, "row must flip to a directory");
        assert_eq!(
            track.parent_id,
            Some(live.id),
            "children must hang off the reinserted directory"
        );
    }

    #[test]
    fn directory_replaced_by_file_drops_its_stale_subtree() {
        let tmp = TempDir::new("flipback");
        write(&tmp.path, "Mixes/one.mp3", b"a");
        write(&tmp.path, "Mixes/two.mp3", b"bb");

        let c = conn();
        let cat = insert_catalog(&c, "t", "/r", "2026-01-01T00:00:00Z").unwrap();
        scan_directory(&c, cat, &tmp.path).unwrap();

        std::fs::remove_dir_all(tmp.path.join("Mixes")).unwrap();
        write(&tmp.path, "Mixes", b"now a file");

        apply_update(&c, cat, &tmp.path).unwrap();

        let entries = get_all_entries(&c, cat).unwrap();
        assert_eq!(entries.len(), 1, "the old children must be gone");
        assert!(!entries[0].is_dir);
        assert_eq!(entries[0].size, 10);
        assert_eq!(get_catalog_by_id(&c, cat).unwrap().total_size, 10);
    }

    fn assert_same_counts(previewed: &UpdatePreview, applied: &UpdatePreview) {
        assert_eq!(previewed.added, applied.added, "added");
        assert_eq!(previewed.updated, applied.updated, "updated");
        assert_eq!(
            previewed.deleted_files, applied.deleted_files,
            "deleted_files"
        );
        assert_eq!(
            previewed.deleted_folders, applied.deleted_folders,
            "deleted_folders"
        );
        assert_eq!(previewed.unchanged, applied.unchanged, "unchanged");
    }

    #[test]
    fn preview_promises_exactly_what_apply_does() {
        let tmp = TempDir::new("agree");
        write(&tmp.path, "keep.txt", b"AAAA");
        write(&tmp.path, "change.txt", b"AAAA");
        write(&tmp.path, "sub/del.txt", b"X");

        let c = conn();
        let cat = insert_catalog(&c, "t", "/r", "2026-01-01T00:00:00Z").unwrap();
        scan_directory(&c, cat, &tmp.path).unwrap();

        write(&tmp.path, "change.txt", b"AAAAAAAA");
        std::fs::remove_file(tmp.path.join("sub/del.txt")).unwrap();
        write(&tmp.path, "new.txt", b"NN");

        let previewed = preview_update(&c, cat, &tmp.path).unwrap();
        let applied = apply_update(&c, cat, &tmp.path).unwrap();

        assert_same_counts(&previewed, &applied);
        assert_eq!(previewed.tags_to_backfill, applied.tags_to_backfill);
        assert_eq!(previewed.added, 1);
        assert_eq!(previewed.deleted_files, 1);
    }

    #[test]
    fn preview_promises_what_apply_does_when_a_directory_becomes_a_file() {
        let tmp = TempDir::new("agreeflip");
        write(&tmp.path, "Mixes/one.txt", b"a");
        write(&tmp.path, "Mixes/two.txt", b"bb");

        let c = conn();
        let cat = insert_catalog(&c, "t", "/r", "2026-01-01T00:00:00Z").unwrap();
        scan_directory(&c, cat, &tmp.path).unwrap();

        std::fs::remove_dir_all(tmp.path.join("Mixes")).unwrap();
        write(&tmp.path, "Mixes", b"now a file");

        let previewed = preview_update(&c, cat, &tmp.path).unwrap();
        let applied = apply_update(&c, cat, &tmp.path).unwrap();

        assert_same_counts(&previewed, &applied);
        assert_eq!(previewed.deleted_folders, 1, "Mixes");
        assert_eq!(previewed.deleted_files, 2, "one.txt + two.txt");
        assert_eq!(previewed.added, 1, "Mixes, now a file");
    }

    #[test]
    fn preview_counts_the_tag_work_a_changed_media_file_will_cost() {
        let tmp = TempDir::new("mediapreview");
        write(&tmp.path, "song.mp3", b"AAAA");

        let c = conn();
        let cat = insert_catalog(&c, "t", "/r", "2026-01-01T00:00:00Z").unwrap();
        scan_directory(&c, cat, &tmp.path).unwrap();

        write(&tmp.path, "song.mp3", b"AAAAAAAA");

        let previewed = preview_update(&c, cat, &tmp.path).unwrap();
        assert_eq!(previewed.updated, 1, "song.mp3 grew");
        assert_eq!(
            previewed.tags_to_backfill, 1,
            "a changed media file has its tags re-read, so the preview must say so"
        );
    }

    fn walked_paths(root: &Path) -> Vec<String> {
        let mut paths: Vec<String> = walk_disk(root, WalkPolicy::Strict)
            .unwrap()
            .into_iter()
            .map(|e| e.rel_path)
            .collect();
        paths.sort();
        paths
    }

    #[test]
    fn walk_disk_reports_paths_relative_to_the_root() {
        let tmp = TempDir::new("walkrel");
        write(&tmp.path, "top.txt", b"T");
        write(&tmp.path, "sub/deep/a.txt", b"A");

        let paths = walked_paths(&tmp.path);
        let got: Vec<&str> = paths.iter().map(String::as_str).collect();

        assert_eq!(got, ["sub", "sub/deep", "sub/deep/a.txt", "top.txt"]);
    }

    #[test]
    fn walk_disk_sizes_files_and_leaves_folders_at_zero() {
        let tmp = TempDir::new("walksize");
        write(&tmp.path, "sub/a.txt", b"12345");

        let entries = walk_disk(&tmp.path, WalkPolicy::Strict).unwrap();
        let dir = entries.iter().find(|e| e.rel_path == "sub").unwrap();
        let file = entries.iter().find(|e| e.rel_path == "sub/a.txt").unwrap();

        assert!(dir.is_dir);
        assert_eq!(dir.size, 0, "folders stay sizeless");
        assert_eq!(dir.extension, None);

        assert!(!file.is_dir);
        assert_eq!(file.size, 5);
        assert_eq!(file.name, "a.txt");
        assert_eq!(file.extension.as_deref(), Some("txt"));
        assert!(file.modified.is_some());
    }

    #[test]
    fn walk_disk_prunes_a_hidden_directorys_whole_subtree() {
        let tmp = TempDir::new("walkhidden");
        write(&tmp.path, "keep.txt", b"K");
        write(&tmp.path, ".git/config", b"C");
        write(&tmp.path, "Thumbs.db", b"junk");

        let paths = walked_paths(&tmp.path);
        let got: Vec<&str> = paths.iter().map(String::as_str).collect();

        assert_eq!(got, ["keep.txt"], "a skipped folder takes its children too");
    }

    #[test]
    fn walk_disk_does_not_descend_into_a_symlinked_directory() {
        use std::os::unix::fs::symlink;

        let tmp = TempDir::new("walklink");
        write(&tmp.path, "real/inner.txt", b"I");
        symlink(tmp.path.join("real"), tmp.path.join("link")).unwrap();

        let paths = walked_paths(&tmp.path);

        assert!(paths.iter().any(|p| p == "real/inner.txt"));
        assert!(
            paths.iter().any(|p| p == "link"),
            "the link itself is listed"
        );
        assert!(
            !paths.iter().any(|p| p.starts_with("link/")),
            "descending a symlink is how a scan ends up walking in circles"
        );
    }
}
