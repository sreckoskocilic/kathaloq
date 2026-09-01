use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, params};
use unicode_normalization::UnicodeNormalization;

use crate::models::{Catalog, FileEntry, FolderStats, MediaTags, SearchResult};

pub fn search_key(value: &str) -> String {
    value
        .nfc()
        .collect::<String>()
        .to_lowercase()
        .nfc()
        .collect()
}

/// Depth cap for the recursive tree CTEs: bounds runaway depth and ends cycles.
const MAX_TREE_DEPTH: u32 = 100;

const MAX_SQL_PARAMS: usize = 30_000;

const SEARCH_LIMIT: i64 = 500;

fn id_json(ids: &[i64]) -> String {
    let mut out = String::with_capacity(ids.len() * 8 + 2);
    out.push('[');
    for (i, id) in ids.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&id.to_string());
    }
    out.push(']');
    out
}

fn u64_at(row: &rusqlite::Row, idx: usize) -> rusqlite::Result<u64> {
    Ok(row.get::<_, i64>(idx)? as u64)
}

pub fn insert_catalog(
    conn: &Connection,
    name: &str,
    root_path: &str,
    scanned_at: &str,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO catalogs (name, root_path, scanned_at) VALUES (?1, ?2, ?3)",
        params![name, root_path, scanned_at],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_catalog_stats(
    conn: &Connection,
    id: i64,
    total_files: u64,
    total_size: u64,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE catalogs SET total_files = ?1, total_size = ?2 WHERE id = ?3",
        params![total_files as i64, total_size as i64, id],
    )?;
    Ok(())
}

pub fn list_catalogs(conn: &Connection) -> rusqlite::Result<Vec<Catalog>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, root_path, scanned_at, total_files, total_size FROM catalogs ORDER BY scanned_at DESC, id DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Catalog {
            id: row.get(0)?,
            name: row.get(1)?,
            root_path: row.get(2)?,
            scanned_at: row.get(3)?,
            total_files: u64_at(row, 4)?,
            total_size: u64_at(row, 5)?,
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
    // Cached stmt: per-file scan hot path, skip re-parsing the INSERT each file.
    conn.prepare_cached(
        "INSERT INTO file_entries (catalog_id, parent_id, name, name_lower, path, is_dir, size, modified, extension)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?
    .execute(params![catalog_id, parent_id, name, search_key(name), path, is_dir as i32, size as i64, modified, extension])?;
    Ok(conn.last_insert_rowid())
}

pub fn get_children(
    conn: &Connection,
    catalog_id: i64,
    parent_id: Option<i64>,
) -> rusqlite::Result<Vec<FileEntry>> {
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

pub fn search_files(
    conn: &Connection,
    catalog_id: i64,
    query: &str,
) -> rusqlite::Result<SearchResult> {
    let escaped = search_key(query)
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let pattern = format!("%{escaped}%");
    let mut stmt = conn.prepare(
        "SELECT id, catalog_id, parent_id, name, path, is_dir, size, modified, extension
         FROM file_entries WHERE catalog_id = ?1 AND name_lower LIKE ?2 ESCAPE '\\'
         ORDER BY is_dir DESC, name_lower ASC LIMIT ?3",
    )?;
    let rows = stmt.query_map(
        params![catalog_id, pattern, SEARCH_LIMIT + 1],
        map_file_entry,
    )?;
    let mut hits: Vec<FileEntry> = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let truncated = hits.len() > SEARCH_LIMIT as usize;
    hits.truncate(SEARCH_LIMIT as usize);
    Ok(SearchResult { hits, truncated })
}

pub fn get_ancestors(
    conn: &Connection,
    catalog_id: i64,
    entry_id: i64,
) -> rusqlite::Result<Vec<FileEntry>> {
    let sql = format!(
        "WITH RECURSIVE chain(id, depth) AS (
            SELECT id, 0 FROM file_entries WHERE catalog_id = ?1 AND id = ?2
            UNION
            SELECT fe.parent_id, c.depth + 1 FROM file_entries fe
            JOIN chain c ON fe.id = c.id
            WHERE fe.parent_id IS NOT NULL AND fe.catalog_id = ?1 AND c.depth < {MAX_TREE_DEPTH}
        )
        SELECT fe.id, fe.catalog_id, fe.parent_id, fe.name, fe.path, fe.is_dir, fe.size, fe.modified, fe.extension
         FROM file_entries fe JOIN chain c ON fe.id = c.id
         WHERE fe.catalog_id = ?1
         ORDER BY c.depth DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![catalog_id, entry_id], map_file_entry)?;
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

pub fn delete_file_entries_by_ids(
    conn: &Connection,
    catalog_id: i64,
    ids: &[i64],
) -> rusqlite::Result<()> {
    for id in ids.iter().rev() {
        conn.execute(
            "DELETE FROM file_entries WHERE id = ?1 AND catalog_id = ?2",
            params![id, catalog_id],
        )?;
    }
    Ok(())
}

pub fn collect_descendant_ids(
    conn: &Connection,
    catalog_id: i64,
    root_ids: &[i64],
) -> rusqlite::Result<Vec<i64>> {
    let mut depth_by_id: HashMap<i64, i64> = HashMap::new();
    for chunk in root_ids.chunks(MAX_SQL_PARAMS) {
        for (id, depth) in collect_descendants_chunk(conn, catalog_id, chunk)? {
            depth_by_id
                .entry(id)
                .and_modify(|d| *d = (*d).max(depth))
                .or_insert(depth);
        }
    }

    let mut pairs: Vec<(i64, i64)> = depth_by_id.into_iter().collect();
    pairs.sort_by_key(|&(id, depth)| (depth, id));
    Ok(pairs.into_iter().map(|(id, _)| id).collect())
}

fn collect_descendants_chunk(
    conn: &Connection,
    catalog_id: i64,
    root_ids: &[i64],
) -> rusqlite::Result<Vec<(i64, i64)>> {
    if root_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders: String = (0..root_ids.len())
        .map(|i| format!("?{}", i + 2))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "WITH RECURSIVE descendants(id, depth) AS (
            SELECT id, 0 FROM file_entries WHERE catalog_id = ?1 AND id IN ({placeholders})
            UNION
            SELECT fe.id, d.depth + 1 FROM file_entries fe
            JOIN descendants d ON fe.parent_id = d.id
            WHERE fe.catalog_id = ?1 AND d.depth < {MAX_TREE_DEPTH}
        )
        SELECT id, MAX(depth) FROM descendants GROUP BY id ORDER BY MAX(depth)",
        MAX_TREE_DEPTH = MAX_TREE_DEPTH,
    );
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(root_ids.len() + 1);
    params.push(&catalog_id);
    for id in root_ids {
        params.push(id as &dyn rusqlite::ToSql);
    }
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params.as_slice(), |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

pub fn recalc_catalog_stats(conn: &Connection, catalog_id: i64) -> rusqlite::Result<()> {
    let (total_files, total_size): (u64, u64) = conn.query_row(
        "SELECT COALESCE(COUNT(*), 0), COALESCE(SUM(size), 0) FROM file_entries WHERE catalog_id = ?1 AND is_dir = 0",
        params![catalog_id],
        |row| Ok((u64_at(row, 0)?, u64_at(row, 1)?)),
    )?;
    update_catalog_stats(conn, catalog_id, total_files, total_size)
}

pub fn update_file_entry_metadata(
    conn: &Connection,
    id: i64,
    size: u64,
    modified: Option<&str>,
) -> rusqlite::Result<()> {
    conn.prepare_cached("UPDATE file_entries SET size = ?1, modified = ?2 WHERE id = ?3")?
        .execute(params![size as i64, modified, id])?;
    Ok(())
}

pub fn update_catalog_scanned_at(
    conn: &Connection,
    id: i64,
    scanned_at: &str,
) -> rusqlite::Result<()> {
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
                total_files: u64_at(row, 4)?,
                total_size: u64_at(row, 5)?,
            })
        },
    )
}

pub fn get_folder_stats(
    conn: &Connection,
    catalog_id: i64,
    folder_id: i64,
) -> rusqlite::Result<FolderStats> {
    // SUM over descendants in one CTE, not id-collect + a query_row per row (old N+1).
    let (file_count, folder_count, total_size): (u64, u64, u64) = conn.query_row(
        "WITH RECURSIVE descendants(id, depth) AS (
            SELECT id, 0 FROM file_entries WHERE id = ?1 AND catalog_id = ?2
            UNION
            SELECT fe.id, d.depth + 1 FROM file_entries fe
            JOIN descendants d ON fe.parent_id = d.id
            WHERE fe.catalog_id = ?2 AND d.depth < ?3
        )
        SELECT
            COALESCE(SUM(CASE WHEN fe.is_dir = 0 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN fe.is_dir = 1 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN fe.is_dir = 0 THEN fe.size ELSE 0 END), 0)
         FROM file_entries fe
         WHERE fe.id IN (SELECT id FROM descendants WHERE id != ?1)",
        params![folder_id, catalog_id, MAX_TREE_DEPTH],
        |row| Ok((u64_at(row, 0)?, u64_at(row, 1)?, u64_at(row, 2)?)),
    )?;

    Ok(FolderStats {
        file_count,
        folder_count,
        total_size,
    })
}

pub fn get_bulk_stats(
    conn: &Connection,
    catalog_id: i64,
    ids: &[i64],
) -> rusqlite::Result<FolderStats> {
    // Coverage set: a selected file counts itself, a selected folder its descendants
    // (not itself), overlaps de-dup. One CTE, not the old per-id query loops.
    if ids.is_empty() {
        return Ok(FolderStats {
            file_count: 0,
            folder_count: 0,
            total_size: 0,
        });
    }
    let sql = "WITH RECURSIVE
           seeds(id, is_dir) AS (
             SELECT fe.id, fe.is_dir FROM file_entries fe
               JOIN json_each(?1) j ON fe.id = j.value
               WHERE fe.catalog_id = ?2
           ),
           covered(id, depth) AS (
             SELECT id, 0 FROM (
               SELECT id FROM seeds WHERE is_dir = 0
               UNION
               SELECT fe.id FROM file_entries fe JOIN seeds s ON fe.parent_id = s.id
                 WHERE s.is_dir = 1 AND fe.catalog_id = ?2
             )
             UNION
             SELECT fe.id, c.depth + 1 FROM file_entries fe JOIN covered c ON fe.parent_id = c.id
               WHERE fe.catalog_id = ?2 AND c.depth < ?3
           )
         SELECT
            COALESCE(SUM(CASE WHEN fe.is_dir = 0 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN fe.is_dir = 1 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN fe.is_dir = 0 THEN fe.size ELSE 0 END), 0)
         FROM file_entries fe
         WHERE fe.id IN (SELECT id FROM covered)";

    let (file_count, folder_count, total_size): (u64, u64, u64) = conn.query_row(
        sql,
        params![id_json(ids), catalog_id, MAX_TREE_DEPTH],
        |row| Ok((u64_at(row, 0)?, u64_at(row, 1)?, u64_at(row, 2)?)),
    )?;

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
    conn.prepare_cached(
        "INSERT OR REPLACE INTO media_tags (file_entry_id, duration_secs, bitrate, sample_rate, channels, title, artist, album, genre, year, track_number)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
    )?
    .execute(params![file_entry_id, duration_secs, bitrate, sample_rate, channels, title, artist, album, genre, year, track_number])?;
    Ok(conn.last_insert_rowid())
}

pub fn get_media_tags(
    conn: &Connection,
    file_entry_id: i64,
) -> rusqlite::Result<Option<MediaTags>> {
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

pub fn get_media_tags_bulk(
    conn: &Connection,
    file_entry_ids: &[i64],
) -> rusqlite::Result<Vec<MediaTags>> {
    if file_entry_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(
        "SELECT id, file_entry_id, duration_secs, bitrate, sample_rate, channels, title, artist, album, genre, year, track_number
         FROM media_tags WHERE file_entry_id IN (SELECT value FROM json_each(?1))",
    )?;
    let rows = stmt.query_map(params![id_json(file_entry_ids)], |row| {
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
    rows.collect()
}

pub fn media_tagged_ids(conn: &Connection, catalog_id: i64) -> rusqlite::Result<HashSet<i64>> {
    let mut stmt = conn.prepare(
        "SELECT mt.file_entry_id FROM media_tags mt
         JOIN file_entries fe ON fe.id = mt.file_entry_id
         WHERE fe.catalog_id = ?1",
    )?;
    let rows = stmt.query_map(params![catalog_id], |row| row.get(0))?;
    rows.collect()
}

pub fn get_children_filtered(
    conn: &Connection,
    catalog_id: i64,
    parent_id: Option<i64>,
    media_type: &str,
) -> rusqlite::Result<Vec<FileEntry>> {
    let extensions: &[&str] = match media_type {
        "audio" => crate::scanner::media::AUDIO_EXTENSIONS,
        "video" => crate::scanner::media::VIDEO_EXTENSIONS,
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
            WHERE fe2.parent_id IS NOT NULL AND fe2.catalog_id = ?1 AND a.depth < {MAX_TREE_DEPTH}
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
        size: u64_at(row, 6)?,
        modified: row.get(7)?,
        extension: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::run_migrations;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run_migrations(&c).unwrap();
        c
    }

    fn catalog(c: &Connection) -> i64 {
        insert_catalog(c, "test", "/root", "2026-01-01T00:00:00Z").unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn dir(c: &Connection, cat: i64, parent: Option<i64>, name: &str) -> i64 {
        insert_file_entry(c, cat, parent, name, name, true, 0, None, None).unwrap()
    }

    fn file(
        c: &Connection,
        cat: i64,
        parent: Option<i64>,
        name: &str,
        size: u64,
        ext: &str,
    ) -> i64 {
        insert_file_entry(c, cat, parent, name, name, false, size, None, Some(ext)).unwrap()
    }

    #[test]
    fn collect_descendant_ids_walks_full_tree() {
        let c = conn();
        let cat = catalog(&c);
        let a = dir(&c, cat, None, "a");
        let b = dir(&c, cat, Some(a), "a/b");
        let f = file(&c, cat, Some(b), "a/b/f.txt", 1, "txt");

        let mut ids = collect_descendant_ids(&c, cat, &[a]).unwrap();
        ids.sort();
        let mut expected = vec![a, b, f];
        expected.sort();
        assert_eq!(ids, expected);
    }

    // Circular parent_id chain must terminate (M1 regression: used to loop forever).
    #[test]
    fn collect_descendant_ids_terminates_on_cycle() {
        let c = conn();
        let cat = catalog(&c);
        let a = dir(&c, cat, None, "a");
        let b = dir(&c, cat, Some(a), "a/b");
        // Make it circular: a↔b.
        c.execute(
            "UPDATE file_entries SET parent_id = ?1 WHERE id = ?2",
            params![b, a],
        )
        .unwrap();

        let mut ids = collect_descendant_ids(&c, cat, &[a]).unwrap();
        ids.sort();
        let mut expected = vec![a, b];
        expected.sort();
        assert_eq!(ids, expected, "each id visited exactly once");
    }

    // rev() delete must hit children before parents (parent_id FK).
    #[test]
    fn delete_file_entries_by_ids_respects_parent_fk() {
        let c = conn();
        let cat = catalog(&c);
        let a = dir(&c, cat, None, "a");
        let b = dir(&c, cat, Some(a), "a/b");
        let _f = file(&c, cat, Some(b), "a/b/f.txt", 1, "txt");

        let ids = collect_descendant_ids(&c, cat, &[a]).unwrap();
        delete_file_entries_by_ids(&c, cat, &ids).expect("delete must not violate FK");
        assert_eq!(get_all_entries(&c, cat).unwrap().len(), 0);
    }

    #[test]
    fn ancestors_return_the_full_chain_root_first() {
        let c = conn();
        let cat = catalog(&c);
        let other = catalog(&c);
        let archive = dir(&c, cat, None, "Archive");
        let photos = dir(&c, cat, Some(archive), "Archive/Photos");
        let year = dir(&c, cat, Some(photos), "Archive/Photos/2019");

        let chain = get_ancestors(&c, cat, year).unwrap();
        let names: Vec<&str> = chain.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["Archive", "Archive/Photos", "Archive/Photos/2019"]
        );

        assert_eq!(get_ancestors(&c, cat, archive).unwrap().len(), 1);
        assert!(get_ancestors(&c, other, year).unwrap().is_empty());
    }

    #[test]
    fn search_matches_macos_decomposed_filenames() {
        let c = conn();
        let cat = catalog(&c);
        let decomposed = "Kres\u{30c}imir Mis\u{30c}ak Sve pis\u{30c}e u novinama .pdf";
        assert!(!decomposed.contains('š'), "fixture must stay decomposed");
        file(&c, cat, None, decomposed, 1, "pdf");

        for query in ["Mišak", "mišak", "Krešimir", "piše"] {
            assert_eq!(
                search_files(&c, cat, query).unwrap().hits.len(),
                1,
                "composed query {query:?} must match a decomposed filename"
            );
        }

        assert_eq!(search_files(&c, cat, "Mis\u{30c}ak").unwrap().hits.len(), 1);
        assert_eq!(search_files(&c, cat, "Misak").unwrap().hits.len(), 0);
    }

    #[test]
    fn migration_recomposes_name_lower_written_before_normalization() {
        let c = conn();
        let cat = catalog(&c);
        let id = file(&c, cat, None, "Kres\u{30c}imir Mis\u{30c}ak.pdf", 1, "pdf");
        c.execute(
            "UPDATE file_entries SET name_lower = ?1 WHERE id = ?2",
            params!["kres\u{30c}imir mis\u{30c}ak.pdf", id],
        )
        .unwrap();
        c.execute_batch("PRAGMA user_version = 0;").unwrap();
        assert_eq!(search_files(&c, cat, "Mišak").unwrap().hits.len(), 0);

        run_migrations(&c).unwrap();

        assert_eq!(
            search_files(&c, cat, "Mišak").unwrap().hits.len(),
            1,
            "an existing catalog must be re-folded on startup"
        );
    }

    #[test]
    fn search_folds_case_for_accented_names() {
        let c = conn();
        let cat = catalog(&c);
        file(&c, cat, None, "Žuti pas.mp3", 1, "mp3");
        file(&c, cat, None, "šarena čaša.flac", 1, "flac");
        file(&c, cat, None, "Zebra.mp3", 1, "mp3");

        for query in ["žuti", "ŽUTI", "Žuti"] {
            let hits = search_files(&c, cat, query).unwrap().hits;
            assert_eq!(hits.len(), 1, "query {query:?} must match Žuti pas.mp3");
            assert_eq!(hits[0].name, "Žuti pas.mp3");
        }

        assert_eq!(search_files(&c, cat, "ČAŠA").unwrap().hits.len(), 1);
        assert_eq!(search_files(&c, cat, "zebra").unwrap().hits.len(), 1);
        assert_eq!(search_files(&c, cat, "đ").unwrap().hits.len(), 0);
    }

    #[test]
    fn migration_backfills_name_lower_for_existing_rows() {
        let c = conn();
        let cat = catalog(&c);
        let id = file(&c, cat, None, "Žuti pas.mp3", 1, "mp3");
        c.execute(
            "UPDATE file_entries SET name_lower = NULL WHERE id = ?1",
            params![id],
        )
        .unwrap();
        assert_eq!(search_files(&c, cat, "žuti").unwrap().hits.len(), 0);

        run_migrations(&c).unwrap();

        assert_eq!(
            search_files(&c, cat, "žuti").unwrap().hits.len(),
            1,
            "rows written before the column existed must be folded on startup"
        );
    }

    #[test]
    fn selecting_a_folder_and_its_own_descendant_deletes_fk_safely() {
        let c = conn();
        let cat = catalog(&c);
        let a = dir(&c, cat, None, "A");
        let b = dir(&c, cat, Some(a), "A/B");
        let f = file(&c, cat, Some(b), "A/B/c.txt", 1, "txt");

        let ids = collect_descendant_ids(&c, cat, &[a, f]).unwrap();
        assert_eq!(
            ids,
            vec![a, b, f],
            "a seeded root that is also a descendant must sort after its parent"
        );
        delete_file_entries_by_ids(&c, cat, &ids)
            .expect("reversed delete must not violate the parent_id FK");
        assert_eq!(get_all_entries(&c, cat).unwrap().len(), 0);
    }

    #[test]
    fn descendant_walk_and_delete_stay_inside_their_catalog() {
        let c = conn();
        let one = catalog(&c);
        let two = catalog(&c);
        let a = dir(&c, one, None, "a");
        let b = dir(&c, one, Some(a), "a/b");
        let f = file(&c, one, Some(b), "a/b/f.txt", 1, "txt");

        assert!(
            collect_descendant_ids(&c, two, &[a]).unwrap().is_empty(),
            "ids from another catalog must not resolve"
        );

        delete_file_entries_by_ids(&c, two, &[f, b, a]).unwrap();
        assert_eq!(
            get_all_entries(&c, one).unwrap().len(),
            3,
            "a stale catalog id must not delete another catalog's rows"
        );
    }

    #[test]
    fn get_children_orders_dirs_first_then_name() {
        let c = conn();
        let cat = catalog(&c);
        file(&c, cat, None, "zebra.txt", 1, "txt");
        file(&c, cat, None, "apple.txt", 1, "txt");
        dir(&c, cat, None, "mid");

        let names: Vec<String> = get_children(&c, cat, None)
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(names, vec!["mid", "apple.txt", "zebra.txt"]);
    }

    #[test]
    fn get_children_filters_by_parent() {
        let c = conn();
        let cat = catalog(&c);
        let d = dir(&c, cat, None, "d");
        file(&c, cat, Some(d), "d/inside.txt", 1, "txt");
        file(&c, cat, None, "root.txt", 1, "txt");

        let root = get_children(&c, cat, None).unwrap();
        assert_eq!(root.len(), 2); // d + root.txt
        let inside = get_children(&c, cat, Some(d)).unwrap();
        assert_eq!(inside.len(), 1);
        assert_eq!(inside[0].name, "d/inside.txt");
    }

    #[test]
    fn get_children_filtered_keeps_audio_and_ancestor_dirs() {
        let c = conn();
        let cat = catalog(&c);
        let music = dir(&c, cat, None, "music");
        file(&c, cat, Some(music), "music/deep.mp3", 1, "mp3");
        file(&c, cat, None, "song.mp3", 1, "mp3");
        file(&c, cat, None, "clip.mp4", 1, "mp4");
        file(&c, cat, None, "notes.txt", 1, "txt");

        let audio: Vec<String> = get_children_filtered(&c, cat, None, "audio")
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        // dir first (ancestor of deep.mp3), then the root-level audio file.
        assert_eq!(audio, vec!["music", "song.mp3"]);

        let video: Vec<String> = get_children_filtered(&c, cat, None, "video")
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(video, vec!["clip.mp4"]);
    }

    #[test]
    fn get_children_filtered_is_catalog_scoped() {
        let c = conn();
        let cat1 = catalog(&c);
        let cat2 = insert_catalog(&c, "other", "/other", "2026-01-01T00:00:00Z").unwrap();
        let d2 = dir(&c, cat2, None, "m");
        file(&c, cat2, Some(d2), "m/x.mp3", 1, "mp3");

        let res = get_children_filtered(&c, cat1, None, "audio").unwrap();
        assert!(res.is_empty(), "cat1 must not see cat2 entries");
    }

    #[test]
    fn get_children_filtered_surfaces_deep_media_ancestors() {
        // Media 3+ levels deep: ancestor CTE must surface intermediate dirs, skip media-less ones.
        let c = conn();
        let cat = catalog(&c);
        let music = dir(&c, cat, None, "music");
        let year = dir(&c, cat, Some(music), "music/2020");
        file(&c, cat, Some(year), "music/2020/song.mp3", 1, "mp3");
        let docs = dir(&c, cat, None, "docs");
        file(&c, cat, Some(docs), "docs/readme.txt", 1, "txt");

        // At root: only `music` (ancestor of the deep mp3); `docs` has no media.
        let root: Vec<String> = get_children_filtered(&c, cat, None, "audio")
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(
            root,
            vec!["music"],
            "deep-media ancestor kept, non-media dir dropped"
        );

        // One level down: the intermediate `music/2020` dir is surfaced.
        let inside: Vec<String> = get_children_filtered(&c, cat, Some(music), "audio")
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(inside, vec!["music/2020"]);
    }

    #[test]
    fn search_files_escapes_like_wildcards() {
        let c = conn();
        let cat = catalog(&c);
        file(&c, cat, None, "100%done.txt", 1, "txt");
        file(&c, cat, None, "100Xdone.txt", 1, "txt");

        let hits: Vec<String> = search_files(&c, cat, "100%")
            .unwrap()
            .hits
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(hits, vec!["100%done.txt"], "% is a literal, not a wildcard");
    }

    #[test]
    fn search_files_is_catalog_scoped() {
        let c = conn();
        let cat1 = catalog(&c);
        let cat2 = insert_catalog(&c, "other", "/o", "2026-01-01T00:00:00Z").unwrap();
        file(&c, cat2, None, "secret.txt", 1, "txt");
        assert!(search_files(&c, cat1, "secret").unwrap().hits.is_empty());
    }

    #[test]
    fn search_files_escapes_underscore_wildcard() {
        // `_` is a LIKE wildcard; escape it so "a_b" doesn't match "axb".
        let c = conn();
        let cat = catalog(&c);
        file(&c, cat, None, "a_b.txt", 1, "txt");
        file(&c, cat, None, "axb.txt", 1, "txt");

        let hits: Vec<String> = search_files(&c, cat, "a_b")
            .unwrap()
            .hits
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(
            hits,
            vec!["a_b.txt"],
            "_ is a literal, not a single-char wildcard"
        );
    }

    #[test]
    fn recalc_catalog_stats_counts_files_only() {
        let c = conn();
        let cat = catalog(&c);
        file(&c, cat, None, "a.txt", 10, "txt");
        file(&c, cat, None, "b.txt", 20, "txt");
        dir(&c, cat, None, "d");

        recalc_catalog_stats(&c, cat).unwrap();
        let cat_row = get_catalog_by_id(&c, cat).unwrap();
        assert_eq!(cat_row.total_files, 2);
        assert_eq!(cat_row.total_size, 30);
    }

    #[test]
    fn get_folder_stats_aggregates_descendants() {
        let c = conn();
        let cat = catalog(&c);
        let a = dir(&c, cat, None, "a");
        file(&c, cat, Some(a), "a/f1.txt", 100, "txt");
        let b = dir(&c, cat, Some(a), "a/b");
        file(&c, cat, Some(b), "a/b/f2.txt", 50, "txt");

        let stats = get_folder_stats(&c, cat, a).unwrap();
        assert_eq!(stats.file_count, 2);
        assert_eq!(stats.folder_count, 1); // b, excluding a itself
        assert_eq!(stats.total_size, 150);
    }

    #[test]
    fn get_bulk_stats_for_folder_counts_descendants() {
        let c = conn();
        let cat = catalog(&c);
        let a = dir(&c, cat, None, "a");
        file(&c, cat, Some(a), "a/f1.txt", 100, "txt");
        let b = dir(&c, cat, Some(a), "a/b");
        file(&c, cat, Some(b), "a/b/f2.txt", 50, "txt");

        let stats = get_bulk_stats(&c, cat, &[a]).unwrap();
        assert_eq!(stats.file_count, 2);
        assert_eq!(stats.folder_count, 1); // b, excluding the selected root a
        assert_eq!(stats.total_size, 150);
    }

    #[test]
    fn get_bulk_stats_counts_selected_files_plus_folder_contents() {
        let c = conn();
        let cat = catalog(&c);
        let a = dir(&c, cat, None, "a");
        file(&c, cat, Some(a), "a/f1.txt", 100, "txt");
        let standalone = file(&c, cat, None, "x.txt", 5, "txt");

        let stats = get_bulk_stats(&c, cat, &[a, standalone]).unwrap();
        assert_eq!(stats.file_count, 2); // a/f1.txt (folder content) + x.txt (selected file)
        assert_eq!(stats.folder_count, 0);
        assert_eq!(stats.total_size, 105);
    }

    // A file selected alongside its containing folder must not be double-counted.
    #[test]
    fn get_bulk_stats_dedupes_overlap() {
        let c = conn();
        let cat = catalog(&c);
        let a = dir(&c, cat, None, "a");
        let f1 = file(&c, cat, Some(a), "a/f1.txt", 100, "txt");

        let stats = get_bulk_stats(&c, cat, &[a, f1]).unwrap();
        assert_eq!(stats.file_count, 1);
        assert_eq!(stats.total_size, 100);
    }

    #[test]
    fn bulk_queries_accept_more_ids_than_the_sqlite_variable_limit() {
        let c = conn();
        let cat = catalog(&c);
        let a = dir(&c, cat, None, "a");
        let f = file(&c, cat, Some(a), "a/f.txt", 100, "txt");
        insert_media_tags(
            &c,
            f,
            Some(1.0),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let mut ids: Vec<i64> = (10_000..60_000).collect();
        ids.push(a);
        ids.push(f);
        assert!(
            ids.len() > 32_766,
            "fixture must exceed SQLITE_MAX_VARIABLE_NUMBER, else it proves nothing"
        );

        let stats =
            get_bulk_stats(&c, cat, &ids).expect("a select-all must not blow the bind limit");
        assert_eq!(stats.file_count, 1);
        assert_eq!(stats.total_size, 100);

        assert_eq!(
            get_media_tags_bulk(&c, &ids)
                .expect("a select-all must not blow the bind limit")
                .len(),
            1
        );
    }

    #[test]
    fn media_tags_roundtrip_and_bulk() {
        let c = conn();
        let cat = catalog(&c);
        let f = file(&c, cat, None, "song.mp3", 1, "mp3");
        insert_media_tags(
            &c,
            f,
            Some(123.5),
            Some(320),
            Some(44100),
            Some(2),
            Some("Title"),
            Some("Artist"),
            Some("Album"),
            Some("Genre"),
            Some(2020),
            Some(3),
        )
        .unwrap();

        let tags = get_media_tags(&c, f).unwrap().expect("tags present");
        assert_eq!(tags.duration_secs, Some(123.5));
        assert_eq!(tags.bitrate, Some(320));
        assert_eq!(tags.title.as_deref(), Some("Title"));
        assert_eq!(tags.track_number, Some(3));

        // Missing file -> None; bulk skips missing ids.
        assert!(get_media_tags(&c, 99999).unwrap().is_none());
        assert_eq!(get_media_tags_bulk(&c, &[f, 99999]).unwrap().len(), 1);
    }
}
