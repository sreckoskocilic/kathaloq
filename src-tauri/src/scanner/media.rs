use std::path::Path;

use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use rusqlite::Connection;

use crate::db::insert_media_tags;

const MEDIA_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "opus", "wav", "aiff", "aif", "m4a", "m4b", "aac", "wma", "ape",
    "wv", "mpc", "mp4", "m4v", "mov",
];

pub fn is_media_file(extension: Option<&str>) -> bool {
    extension
        .map(|e| MEDIA_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn extract_and_store_tags(conn: &Connection, file_entry_id: i64, file_path: &Path) -> bool {
    let tagged_file = match Probe::open(file_path).and_then(|p| p.read()) {
        Ok(f) => f,
        Err(_) => {
            // Don't persist a placeholder row on read failure: a NULL row would
            // make get_media_tags() return Some, permanently masking the file
            // from re-tagging once it becomes readable. Leave it untagged.
            return false;
        }
    };

    let props = tagged_file.properties();
    let duration_secs = {
        let d = props.duration();
        let secs = d.as_secs_f64();
        if secs > 0.0 { Some(secs) } else { None }
    };
    let bitrate = props.audio_bitrate();
    let sample_rate = props.sample_rate();
    let channels = props.channels();

    let mut title: Option<String> = None;
    let mut artist: Option<String> = None;
    let mut album: Option<String> = None;
    let mut genre: Option<String> = None;
    let mut year: Option<u32> = None;
    let mut track_number: Option<u32> = None;

    if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
        title = tag.title().map(|s| s.to_string());
        artist = tag.artist().map(|s| s.to_string());
        album = tag.album().map(|s| s.to_string());
        genre = tag.genre().map(|s| s.to_string());
        year = tag.year();
        track_number = tag.track();
    }

    match insert_media_tags(
        conn,
        file_entry_id,
        duration_secs,
        bitrate,
        sample_rate,
        channels,
        title.as_deref(),
        artist.as_deref(),
        album.as_deref(),
        genre.as_deref(),
        year,
        track_number,
    ) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("db: failed to insert media tags for {:?}: {}", file_path, e);
            false
        }
    }
}
