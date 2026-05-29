use std::collections::HashSet;

use rusqlite::{params, Connection};

use crate::models::{Catalog, FileEntry, FolderStats, MediaTags};

pub fn insert_catalog(conn: &Connection, name: &str, root_path: &str, scanned_at: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO catalogs (name, root_path, scanned_at) VALUES (?1, ?2, ?3)",
        params![name, root_path, scanned_at],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_catalog_stats(conn: &Connection, id: i64, total_files: u64, total_size: u64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE catalogs SET total_files = ?1, total_size = ?2 WHERE id = ?3",
        params![total_files, total_size, id],
    )?;
    Ok(())
}

pub fn list_catalogs(conn: &Connection) -> rusqlite::Result<Vec<Catalog>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, root_path, scanned_at, total_files, total_size FROM catalogs ORDER BY scanned_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Catalog {
            id: row.get(0)?,
            name: row.get(1)?,
            root_path: row.get(2)?,
            scanned_at: row.get(3)?,
            total_files: row.get(4)?,
            total_size: row.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn delete_catalog(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM catalogs WHERE id = ?1", params![id])?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn insert_file_entry(
    conn: &Connection,
    catalog_id: i64,
    parent_id: Option<i64>,
    name: &str,
    path: &str,
    is_dir: bool,
    size: u64,
    modified: Option<&str>,
    extension: Option<&str>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO file_entries (catalog_id, parent_id, name, path, is_dir, size, modified, extension)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![catalog_id, parent_id, name, path, is_dir as i32, size, modified, extension],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_children(conn: &Connection, catalog_id: i64, parent_id: Option<i64>) -> rusqlite::Result<Vec<FileEntry>> {
    let mut stmt = if parent_id.is_some() {
        conn.prepare(
            "SELECT id, catalog_id, parent_id, name, path, is_dir, size, modified, extension
             FROM file_entries WHERE catalog_id = ?1 AND parent_id = ?2
             ORDER BY is_dir DESC, name ASC",
        )?
    } else {
        conn.prepare(
            "SELECT id, catalog_id, parent_id, name, path, is_dir, size, modified, extension
             FROM file_entries WHERE catalog_id = ?1 AND parent_id IS NULL
             ORDER BY is_dir DESC, name ASC",
        )?
    };

    let rows = if let Some(pid) = parent_id {
        stmt.query_map(params![catalog_id, pid], map_file_entry)?
    } else {
        stmt.query_map(params![catalog_id], map_file_entry)?
    };
    rows.collect()
}

pub fn search_files(conn: &Connection, catalog_id: i64, query: &str) -> rusqlite::Result<Vec<FileEntry>> {
    let escaped = query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    let pattern = format!("%{escaped}%");
    let mut stmt = conn.prepare(
        "SELECT id, catalog_id, parent_id, name, path, is_dir, size, modified, extension
         FROM file_entries WHERE catalog_id = ?1 AND name LIKE ?2 ESCAPE '\\'
         ORDER BY is_dir DESC, name ASC LIMIT 500",
    )?;
    let rows = stmt.query_map(params![catalog_id, pattern], map_file_entry)?;
    rows.collect()
}

pub fn get_all_entries(conn: &Connection, catalog_id: i64) -> rusqlite::Result<Vec<FileEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, catalog_id, parent_id, name, path, is_dir, size, modified, extension
         FROM file_entries WHERE catalog_id = ?1",
    )?;
    let rows = stmt.query_map(params![catalog_id], map_file_entry)?;
    rows.collect()
}

pub fn delete_file_entries_by_ids(conn: &Connection, ids: &[i64]) -> rusqlite::Result<()> {
    for id in ids.iter().rev() {
        conn.execute("DELETE FROM file_entries WHERE id = ?1", params![id])?;
    }
    Ok(())
}

pub fn collect_descendant_ids(conn: &Connection, root_ids: &[i64]) -> rusqlite::Result<Vec<i64>> {
    let mut all_ids: Vec<i64> = root_ids.to_vec();
    let mut seen: HashSet<i64> = root_ids.iter().copied().collect();
    let mut queue: Vec<i64> = root_ids.to_vec();

    while !queue.is_empty() {
        let current = queue.clone();
        queue.clear();
        for parent_id in current {
            let mut stmt = conn.prepare(
                "SELECT id FROM file_entries WHERE parent_id = ?1",
            )?;
            let children: Vec<i64> = stmt
                .query_map(params![parent_id], |row| row.get(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            // Guard against circular parent_id chains (corrupt data): only
            // visit each id once, otherwise the walk loops forever / OOMs.
            for child in children {
                if seen.insert(child) {
                    all_ids.push(child);
                    queue.push(child);
                }
            }
        }
    }
    Ok(all_ids)
}

pub fn recalc_catalog_stats(conn: &Connection, catalog_id: i64) -> rusqlite::Result<()> {
    let (total_files, total_size): (u64, u64) = conn.query_row(
        "SELECT COALESCE(COUNT(*), 0), COALESCE(SUM(size), 0) FROM file_entries WHERE catalog_id = ?1 AND is_dir = 0",
        params![catalog_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    update_catalog_stats(conn, catalog_id, total_files, total_size)
}

pub fn update_file_entry_metadata(
    conn: &Connection,
    id: i64,
    size: u64,
    modified: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE file_entries SET size = ?1, modified = ?2 WHERE id = ?3",
        params![size, modified, id],
    )?;
    Ok(())
}

pub fn update_catalog_scanned_at(conn: &Connection, id: i64, scanned_at: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE catalogs SET scanned_at = ?1 WHERE id = ?2",
        params![scanned_at, id],
    )?;
    Ok(())
}

pub fn get_catalog_by_id(conn: &Connection, id: i64) -> rusqlite::Result<Catalog> {
    conn.query_row(
        "SELECT id, name, root_path, scanned_at, total_files, total_size FROM catalogs WHERE id = ?1",
        params![id],
        |row| {
            Ok(Catalog {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
                scanned_at: row.get(3)?,
                total_files: row.get(4)?,
                total_size: row.get(5)?,
            })
        },
    )
}

pub fn get_folder_stats(conn: &Connection, catalog_id: i64, folder_id: i64) -> rusqlite::Result<FolderStats> {
    let all_ids = collect_descendant_ids(conn, &[folder_id])?;
    let mut file_count: u64 = 0;
    let mut folder_count: u64 = 0;
    let mut total_size: u64 = 0;

    for id in &all_ids {
        if *id == folder_id {
            continue;
        }
        let (is_dir, size): (bool, u64) = conn.query_row(
            "SELECT is_dir, size FROM file_entries WHERE id = ?1 AND catalog_id = ?2",
            params![id, catalog_id],
            |row| Ok((row.get::<_, i32>(0)? != 0, row.get(1)?)),
        )?;
        if is_dir {
            folder_count += 1;
        } else {
            file_count += 1;
            total_size += size;
        }
    }

    Ok(FolderStats {
        file_count,
        folder_count,
        total_size,
    })
}

pub fn get_bulk_stats(conn: &Connection, catalog_id: i64, ids: &[i64]) -> rusqlite::Result<FolderStats> {
    let mut file_count: u64 = 0;
    let mut folder_count: u64 = 0;
    let mut total_size: u64 = 0;

    let root_ids: std::collections::HashSet<i64> = ids.iter().copied().collect();
    let mut all_ids: Vec<i64> = Vec::new();
    for &id in ids {
        let is_dir: bool = conn.query_row(
            "SELECT is_dir FROM file_entries WHERE id = ?1 AND catalog_id = ?2",
            params![id, catalog_id],
            |row| Ok(row.get::<_, i32>(0)? != 0),
        )?;
        if is_dir {
            let descendants = collect_descendant_ids(conn, &[id])?;
            all_ids.extend(descendants);
        } else {
            all_ids.push(id);
        }
    }

    let unique: std::collections::HashSet<i64> = all_ids.into_iter().collect();
    for id in unique {
        if root_ids.contains(&id) {
            continue;
        }
        let (is_dir, size): (bool, u64) = conn.query_row(
            "SELECT is_dir, size FROM file_entries WHERE id = ?1 AND catalog_id = ?2",
            params![id, catalog_id],
            |row| Ok((row.get::<_, i32>(0)? != 0, row.get(1)?)),
        )?;
        if is_dir {
            folder_count += 1;
        } else {
            file_count += 1;
            total_size += size;
        }
    }

    Ok(FolderStats {
        file_count,
        folder_count,
        total_size,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn insert_media_tags(
    conn: &Connection,
    file_entry_id: i64,
    duration_secs: Option<f64>,
    bitrate: Option<u32>,
    sample_rate: Option<u32>,
    channels: Option<u8>,
    title: Option<&str>,
    artist: Option<&str>,
    album: Option<&str>,
    genre: Option<&str>,
    year: Option<u32>,
    track_number: Option<u32>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT OR REPLACE INTO media_tags (file_entry_id, duration_secs, bitrate, sample_rate, channels, title, artist, album, genre, year, track_number)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![file_entry_id, duration_secs, bitrate, sample_rate, channels, title, artist, album, genre, year, track_number],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_media_tags(conn: &Connection, file_entry_id: i64) -> rusqlite::Result<Option<MediaTags>> {
    let mut stmt = conn.prepare(
        "SELECT id, file_entry_id, duration_secs, bitrate, sample_rate, channels, title, artist, album, genre, year, track_number
         FROM media_tags WHERE file_entry_id = ?1",
    )?;
    let mut rows = stmt.query_map(params![file_entry_id], |row| {
        Ok(MediaTags {
            id: row.get(0)?,
            file_entry_id: row.get(1)?,
            duration_secs: row.get(2)?,
            bitrate: row.get(3)?,
            sample_rate: row.get(4)?,
            channels: row.get(5)?,
            title: row.get(6)?,
            artist: row.get(7)?,
            album: row.get(8)?,
            genre: row.get(9)?,
            year: row.get(10)?,
            track_number: row.get(11)?,
        })
    })?;
    match rows.next() {
        Some(Ok(tags)) => Ok(Some(tags)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

pub fn get_media_tags_bulk(conn: &Connection, file_entry_ids: &[i64]) -> rusqlite::Result<Vec<MediaTags>> {
    let mut result = Vec::new();
    for &id in file_entry_ids {
        if let Some(tags) = get_media_tags(conn, id)? {
            result.push(tags);
        }
    }
    Ok(result)
}

pub fn get_children_filtered(
    conn: &Connection,
    catalog_id: i64,
    parent_id: Option<i64>,
    media_type: &str,
) -> rusqlite::Result<Vec<FileEntry>> {
    let extensions: &[&str] = match media_type {
        "audio" => &["mp3", "flac", "ogg", "opus", "wav", "aiff", "aif", "m4a", "m4b", "aac", "wma", "ape", "wv", "mpc"],
        "video" => &["mp4", "m4v", "mov", "avi", "mkv", "wmv", "webm", "flv"],
        _ => return get_children(conn, catalog_id, parent_id),
    };

    let parent_clause = if parent_id.is_some() {
        "fe.parent_id = ?2"
    } else {
        "fe.parent_id IS NULL"
    };
    let offset: usize = if parent_id.is_some() { 3 } else { 2 };
    let ext_list: String = extensions
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", offset + i))
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!(
        "WITH RECURSIVE ancestors(id, depth) AS (
            SELECT parent_id, 1 FROM file_entries
            WHERE catalog_id = ?1 AND is_dir = 0 AND LOWER(extension) IN ({ext_list})
            AND parent_id IS NOT NULL
            UNION
            SELECT fe2.parent_id, a.depth + 1 FROM file_entries fe2
            INNER JOIN ancestors a ON fe2.id = a.id
            WHERE fe2.parent_id IS NOT NULL AND fe2.catalog_id = ?1 AND a.depth < 100
        )
        SELECT fe.id, fe.catalog_id, fe.parent_id, fe.name, fe.path, fe.is_dir, fe.size, fe.modified, fe.extension
        FROM file_entries fe
        WHERE fe.catalog_id = ?1
          AND {parent_clause}
          AND (
            (fe.is_dir = 0 AND LOWER(fe.extension) IN ({ext_list}))
            OR
            (fe.is_dir = 1 AND fe.id IN (SELECT id FROM ancestors))
          )
        ORDER BY fe.is_dir DESC, fe.name ASC"
    );

    let mut stmt = conn.prepare(&sql)?;
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    params.push(Box::new(catalog_id));
    if let Some(pid) = parent_id {
        params.push(Box::new(pid));
    }
    for ext in extensions {
        params.push(Box::new(ext.to_string()));
    }
    let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|b| b.as_ref()).collect();
    let rows = stmt.query_map(refs.as_slice(), map_file_entry)?;
    rows.collect()
}

fn map_file_entry(row: &rusqlite::Row) -> rusqlite::Result<FileEntry> {
    Ok(FileEntry {
        id: row.get(0)?,
        catalog_id: row.get(1)?,
        parent_id: row.get(2)?,
        name: row.get(3)?,
        path: row.get(4)?,
        is_dir: row.get::<_, i32>(5)? != 0,
        size: row.get(6)?,
        modified: row.get(7)?,
        extension: row.get(8)?,
    })
}
