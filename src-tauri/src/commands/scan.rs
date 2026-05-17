use std::path::PathBuf;

use chrono::Utc;
use tauri::State;

use crate::db::{self, Database};
use crate::models::UpdatePreview;
use crate::scanner::{apply_update, preview_update, scan_directory};

#[tauri::command]
pub async fn start_scan(db: State<'_, Database>, path: String, name: String) -> Result<i64, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }

    let db = db.inner().clone();
    let scanned_at = Utc::now().to_rfc3339();

    tokio::task::spawn_blocking(move || {
        let conn = db.lock();
        let catalog_id =
            db::insert_catalog(&conn, &name, &path, &scanned_at).map_err(|e| e.to_string())?;
        scan_directory(&conn, catalog_id, &root)?;
        Ok(catalog_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn preview_catalog_update(db: State<'_, Database>, catalog_id: i64) -> Result<UpdatePreview, String> {
    let db = db.inner().clone();

    tokio::task::spawn_blocking(move || {
        let conn = db.lock();
        let catalog = db::get_catalog_by_id(&conn, catalog_id).map_err(|e| e.to_string())?;
        let root = PathBuf::from(&catalog.root_path);
        if !root.is_dir() {
            return Err(format!("Path not available: {}", catalog.root_path));
        }
        preview_update(&conn, catalog_id, &root)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apply_catalog_update(db: State<'_, Database>, catalog_id: i64) -> Result<UpdatePreview, String> {
    let db = db.inner().clone();

    tokio::task::spawn_blocking(move || {
        let conn = db.lock();
        let catalog = db::get_catalog_by_id(&conn, catalog_id).map_err(|e| e.to_string())?;
        let root = PathBuf::from(&catalog.root_path);
        if !root.is_dir() {
            return Err(format!("Path not available: {}", catalog.root_path));
        }
        apply_update(&conn, catalog_id, &root)
    })
    .await
    .map_err(|e| e.to_string())?
}
