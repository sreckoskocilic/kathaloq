use std::path::PathBuf;

use chrono::Utc;
use tauri::State;

use crate::db::{self, Database};
use crate::models::UpdatePreview;
use crate::scanner::{
    WalkPolicy, apply_update_entries, index_entries, preview_update_entries, walk_disk,
};

#[tauri::command]
pub async fn start_scan(
    db: State<'_, Database>,
    path: String,
    name: String,
) -> Result<i64, String> {
    let root = PathBuf::from(&path)
        .canonicalize()
        .map_err(|e| format!("Invalid path: {e}"))?;
    if !root.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }

    let db = db.inner().clone();
    let scanned_at = Utc::now().to_rfc3339();

    let root_str = root.to_string_lossy().to_string();

    tokio::task::spawn_blocking(move || {
        let entries = walk_disk(&root, WalkPolicy::SkipUnreadable)?;

        let mut conn = db.lock();
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let catalog_id =
            db::insert_catalog(&tx, &name, &root_str, &scanned_at).map_err(|e| e.to_string())?;
        index_entries(&tx, catalog_id, &entries)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(catalog_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn preview_catalog_update(
    db: State<'_, Database>,
    catalog_id: i64,
) -> Result<UpdatePreview, String> {
    let db = db.inner().clone();

    tokio::task::spawn_blocking(move || {
        let root = catalog_root(&db, catalog_id)?;
        let entries = walk_disk(&root, WalkPolicy::Strict)?;

        let conn = db.read_lock();
        preview_update_entries(&conn, catalog_id, &entries)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn catalog_root(db: &Database, catalog_id: i64) -> Result<PathBuf, String> {
    let conn = db.read_lock();
    let catalog = db::get_catalog_by_id(&conn, catalog_id).map_err(|e| e.to_string())?;
    drop(conn);

    let root = PathBuf::from(&catalog.root_path)
        .canonicalize()
        .map_err(|_| format!("Path not available: {}", catalog.root_path))?;
    if !root.is_dir() {
        return Err(format!("Path not available: {}", catalog.root_path));
    }
    Ok(root)
}

#[tauri::command]
pub async fn apply_catalog_update(
    db: State<'_, Database>,
    catalog_id: i64,
) -> Result<UpdatePreview, String> {
    let db = db.inner().clone();

    tokio::task::spawn_blocking(move || {
        let root = catalog_root(&db, catalog_id)?;
        let entries = walk_disk(&root, WalkPolicy::Strict)?;

        let mut conn = db.lock();
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let result = apply_update_entries(&tx, catalog_id, &entries)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}
