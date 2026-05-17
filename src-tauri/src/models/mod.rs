use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub id: i64,
    pub name: String,
    pub root_path: String,
    pub scanned_at: String,
    pub total_files: u64,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub id: i64,
    pub catalog_id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<String>,
    pub extension: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub files_scanned: u64,
    pub current_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaTags {
    pub id: i64,
    pub file_entry_id: i64,
    pub duration_secs: Option<f64>,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderStats {
    pub file_count: u64,
    pub folder_count: u64,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePreview {
    pub added: u64,
    pub updated: u64,
    pub deleted_files: u64,
    pub deleted_folders: u64,
    pub unchanged: u64,
    pub tags_to_backfill: u64,
}
