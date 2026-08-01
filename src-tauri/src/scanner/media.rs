use std::path::Path;

use lofty::error::ErrorKind;
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

pub fn extract_and_store_tags(
    conn: &Connection,
    file_entry_id: i64,
    file_path: &Path,
) -> Result<bool, String> {
    let tagged_file = match Probe::open(file_path).and_then(|p| p.read()) {
        Ok(f) => f,
        Err(e) => {
            if matches!(e.kind(), ErrorKind::Io(_)) {
                return Ok(false);
            }
            insert_media_tags(
                conn,
                file_entry_id,
                None,
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
            .map_err(|e| e.to_string())?;
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

    if let Some(tag) = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())
    {
        title = tag.title().map(|s| s.to_string());
        artist = tag.artist().map(|s| s.to_string());
        album = tag.album().map(|s| s.to_string());
        genre = tag.genre().map(|s| s.to_string());
        year = tag.date().map(|d| u32::from(d.year));
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
        Err(e) => Err(format!(
            "failed to insert media tags for {file_path:?}: {e}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_and_store_tags, is_media_file};
    use crate::db::{get_media_tags, insert_catalog, insert_file_entry, run_migrations};
    use rusqlite::Connection;
    use std::path::PathBuf;

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

    struct TempFile {
        path: PathBuf,
    }

    impl TempFile {
        fn new(name: &str, bytes: &[u8]) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "kathaloq-media-{}-{nanos}-{name}",
                std::process::id()
            ));
            std::fs::write(&path, bytes).unwrap();
            TempFile { path }
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    fn wav_bytes(frames: u32) -> Vec<u8> {
        let channels: u16 = 2;
        let sample_rate: u32 = 44100;
        let bits: u16 = 16;
        let block_align: u16 = channels * bits / 8;
        let byte_rate: u32 = sample_rate * u32::from(block_align);
        let data_len: u32 = frames * u32::from(block_align);

        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + data_len).to_le_bytes());
        b.extend_from_slice(b"WAVE");
        b.extend_from_slice(b"fmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&channels.to_le_bytes());
        b.extend_from_slice(&sample_rate.to_le_bytes());
        b.extend_from_slice(&byte_rate.to_le_bytes());
        b.extend_from_slice(&block_align.to_le_bytes());
        b.extend_from_slice(&bits.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&data_len.to_le_bytes());
        b.resize(b.len() + data_len as usize, 0);
        b
    }

    fn db_with_file(ext: &str) -> (Connection, i64) {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run_migrations(&c).unwrap();
        let cat = insert_catalog(&c, "t", "/r", "2026-01-01T00:00:00Z").unwrap();
        let name = format!("song.{ext}");
        let id = insert_file_entry(&c, cat, None, &name, &name, false, 1, None, Some(ext)).unwrap();
        (c, id)
    }

    #[test]
    fn a_readable_wav_stores_its_audio_properties() {
        let file = TempFile::new("song.wav", &wav_bytes(4410));
        let (c, id) = db_with_file("wav");

        let stored = extract_and_store_tags(&c, id, &file.path).unwrap();
        assert!(stored, "a parseable file reports that it stored tags");

        let tags = get_media_tags(&c, id).unwrap().expect("a row was written");
        assert_eq!(tags.sample_rate, Some(44100));
        assert_eq!(tags.channels, Some(2));
        let secs = tags.duration_secs.expect("duration was read");
        assert!((secs - 0.1).abs() < 0.01, "0.1s of frames, got {secs}");
    }

    #[test]
    fn an_unparseable_media_file_is_marked_so_the_next_scan_skips_it() {
        let file = TempFile::new("broken.mp3", &vec![0xAAu8; 4096]);
        let (c, id) = db_with_file("mp3");

        let stored = extract_and_store_tags(&c, id, &file.path).unwrap();
        assert!(!stored, "nothing was actually read off the file");

        assert!(
            get_media_tags(&c, id).unwrap().is_some(),
            "an empty row must stand in, or every later scan re-reads the same broken file"
        );
    }

    #[test]
    fn an_unreadable_file_stays_retryable() {
        let (c, id) = db_with_file("mp3");
        let missing = std::env::temp_dir().join("kathaloq-media-does-not-exist.mp3");

        let stored = extract_and_store_tags(&c, id, &missing).unwrap();
        assert!(!stored);
        assert!(
            get_media_tags(&c, id).unwrap().is_none(),
            "a file that was merely unreachable must not be written off"
        );
    }
}
