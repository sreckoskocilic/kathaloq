use tauri::State;

use crate::db::{self, Database};
use crate::models::{Catalog, FileEntry, FolderStats, MediaTags};

#[tauri::command]
pub fn list_catalogs(db: State<Database>) -> Result<Vec<Catalog>, String> {
    db.with_conn(db::list_catalogs).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_catalog(db: State<Database>, id: i64) -> Result<(), String> {
    db.with_conn(|conn| db::delete_catalog(conn, id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_children(db: State<Database>, catalog_id: i64, parent_id: Option<i64>) -> Result<Vec<FileEntry>, String> {
    db.with_conn(|conn| db::get_children(conn, catalog_id, parent_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_files(db: State<Database>, catalog_id: i64, query: String) -> Result<Vec<FileEntry>, String> {
    db.with_conn(|conn| db::search_files(conn, catalog_id, &query))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_folder_stats(db: State<Database>, catalog_id: i64, folder_id: i64) -> Result<FolderStats, String> {
    db.with_conn(|conn| db::get_folder_stats(conn, catalog_id, folder_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_bulk_stats(db: State<Database>, catalog_id: i64, ids: Vec<i64>) -> Result<FolderStats, String> {
    db.with_conn(|conn| db::get_bulk_stats(conn, catalog_id, &ids))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_tags(db: State<Database>, file_entry_id: i64) -> Result<Option<MediaTags>, String> {
    db.with_conn(|conn| db::get_media_tags(conn, file_entry_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_tags_bulk(db: State<Database>, file_entry_ids: Vec<i64>) -> Result<Vec<MediaTags>, String> {
    db.with_conn(|conn| db::get_media_tags_bulk(conn, &file_entry_ids))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_file_entries(db: State<Database>, catalog_id: i64, ids: Vec<i64>) -> Result<(), String> {
    db.with_conn(|conn| {
        let all_ids = db::collect_descendant_ids(conn, &ids)?;
        db::delete_file_entries_by_ids(conn, &all_ids)?;
        db::recalc_catalog_stats(conn, catalog_id)?;
        Ok(())
    })
    .map_err(|e| e.to_string())
}
