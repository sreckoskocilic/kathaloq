use tauri::State;

use crate::db::{self, Database};
use crate::models::{Catalog, FileEntry, FolderStats, MediaTags};

// All query commands are async + spawn_blocking so the SQLite work runs off the
// main thread. A synchronous #[tauri::command] runs on Tauri's main thread and
// blocks the window event loop — a large multi-select (get_bulk_stats walks every
// descendant) would otherwise freeze clicks, double-clicks and the context menu.

#[tauri::command]
pub async fn list_catalogs(db: State<'_, Database>) -> Result<Vec<Catalog>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        db.with_read_conn(db::list_catalogs).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_catalog(db: State<'_, Database>, id: i64) -> Result<(), String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        db.with_conn(|conn| db::delete_catalog(conn, id))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_children(
    db: State<'_, Database>,
    catalog_id: i64,
    parent_id: Option<i64>,
) -> Result<Vec<FileEntry>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        db.with_read_conn(|conn| db::get_children(conn, catalog_id, parent_id))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn search_files(
    db: State<'_, Database>,
    catalog_id: i64,
    query: String,
) -> Result<Vec<FileEntry>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        db.with_read_conn(|conn| db::search_files(conn, catalog_id, &query))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_folder_stats(
    db: State<'_, Database>,
    catalog_id: i64,
    folder_id: i64,
) -> Result<FolderStats, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        db.with_read_conn(|conn| db::get_folder_stats(conn, catalog_id, folder_id))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_bulk_stats(
    db: State<'_, Database>,
    catalog_id: i64,
    ids: Vec<i64>,
) -> Result<FolderStats, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        db.with_read_conn(|conn| db::get_bulk_stats(conn, catalog_id, &ids))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_media_tags(
    db: State<'_, Database>,
    file_entry_id: i64,
) -> Result<Option<MediaTags>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        db.with_read_conn(|conn| db::get_media_tags(conn, file_entry_id))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_media_tags_bulk(
    db: State<'_, Database>,
    file_entry_ids: Vec<i64>,
) -> Result<Vec<MediaTags>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        db.with_read_conn(|conn| db::get_media_tags_bulk(conn, &file_entry_ids))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_children_filtered(
    db: State<'_, Database>,
    catalog_id: i64,
    parent_id: Option<i64>,
    media_type: String,
) -> Result<Vec<FileEntry>, String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        db.with_read_conn(|conn| db::get_children_filtered(conn, catalog_id, parent_id, &media_type))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn remove_file_entries(
    db: State<'_, Database>,
    catalog_id: i64,
    ids: Vec<i64>,
) -> Result<(), String> {
    let db = db.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = db.lock();
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let all_ids = db::collect_descendant_ids(&tx, &ids).map_err(|e| e.to_string())?;
        db::delete_file_entries_by_ids(&tx, &all_ids).map_err(|e| e.to_string())?;
        db::recalc_catalog_stats(&tx, catalog_id).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
