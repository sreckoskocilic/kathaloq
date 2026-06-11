use std::path::Path;

use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use rusqlite::Connection;

use crate::db::insert_media_tags;

// Single source of truth for the media-filter extension sets. The audio/video
// filters (db::get_children_filtered) and tag extraction (is_media_file) share
// these so a format can never appear in a filter yet be ignored for tagging.
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "opus", "wav", "aiff", "aif", "m4a", "m4b", "aac", "wma", "ape", "wv",
    "mpc",
];
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "m4v", "mov", "avi", "mkv", "wmv", "webm", "flv"];

pub fn is_media_file(extension: Option<&str>) -> bool {
    extension
        .map(|e| {
            let e = e.to_lowercase();
            AUDIO_EXTENSIONS.contains(&e.as_str()) || VIDEO_EXTENSIONS.contains(&e.as_str())
        })
        .unwrap_or(false)
}

/// `Ok(true)` stored, `Ok(false)` unparseable (no tags — not an error), `Err` only on a
/// DB failure so the caller's scan tx aborts instead of committing a tagless row.
pub fn extract_and_store_tags(
    conn: &Connection,
    file_entry_id: i64,
    file_path: &Path,
) -> Result<bool, String> {
    let tagged_file = match Probe::open(file_path).and_then(|p| p.read()) {
        Ok(f) => f,
        Err(_) => {
            return Ok(false);
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
        Ok(_) => Ok(true),
        Err(e) => Err(format!("failed to insert media tags for {file_path:?}: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::is_media_file;

    #[test]
    fn recognizes_media_extensions() {
        assert!(is_media_file(Some("mp3")));
        assert!(is_media_file(Some("MP3"))); // case-insensitive
        assert!(is_media_file(Some("flac")));
        assert!(is_media_file(Some("mp4")));
        assert!(is_media_file(Some("mkv"))); // video set, shared with the video filter
        assert!(is_media_file(Some("webm")));
    }

    #[test]
    fn rejects_non_media() {
        assert!(!is_media_file(Some("txt")));
        assert!(!is_media_file(Some("")));
        assert!(!is_media_file(None));
    }
}
