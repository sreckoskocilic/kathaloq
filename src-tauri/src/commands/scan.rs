use std::path::PathBuf;

use chrono::Utc;
use tauri::State;

use crate::db::{self, Database};
use crate::models::UpdatePreview;
use crate::scanner::{apply_update, preview_update, scan_directory};

#[tauri::command]
pub async fn start_scan(db: State<'_, Database>, path: String, name: String) -> Result<i64, String> {
    let root = PathBuf::from(&path).canonicalize().map_err(|e| format!("Invalid path: {e}"))?;
    if !root.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }

    let db = db.inner().clone();
    let scanned_at = Utc::now().to_rfc3339();

    let root_str = root.to_string_lossy().to_string();

    tokio::task::spawn_blocking(move || {
        let mut conn = db.lock();
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let catalog_id =
            db::insert_catalog(&tx, &name, &root_str, &scanned_at).map_err(|e| e.to_string())?;
        scan_directory(&tx, catalog_id, &root)?;
        tx.commit().map_err(|e| e.to_string())?;
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
        let mut conn = db.lock();
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let catalog = db::get_catalog_by_id(&tx, catalog_id).map_err(|e| e.to_string())?;
        let root = PathBuf::from(&catalog.root_path);
        if !root.is_dir() {
            return Err(format!("Path not available: {}", catalog.root_path));
        }
        let result = apply_update(&tx, catalog_id, &root)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}
