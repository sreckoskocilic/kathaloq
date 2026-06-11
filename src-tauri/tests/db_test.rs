use rusqlite::Connection;

// Integration tests for behaviors with no stronger in-crate (queries.rs) equivalent:
// catalog list/cascade, exhaustive media-tag column roundtrip, media-tag FK cascade,
// and metadata update. Path/stats/search/descendant/media-classification behavior is
// covered (more thoroughly) by the inline #[cfg(test)] suites in queries.rs and media.rs.

fn in_memory_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    kathaloq_lib::db::run_migrations(&conn).unwrap();
    conn
}

#[test]
fn test_insert_and_list_catalogs() {
    let conn = in_memory_db();

    let id = kathaloq_lib::db::insert_catalog(&conn, "Test", "/tmp/test", "2024-01-01T00:00:00Z")
        .unwrap();
    assert!(id > 0);

    let catalogs = kathaloq_lib::db::list_catalogs(&conn).unwrap();
    assert_eq!(catalogs.len(), 1);
    assert_eq!(catalogs[0].name, "Test");
    assert_eq!(catalogs[0].root_path, "/tmp/test");
}

#[test]
fn test_delete_catalog_cascades() {
    let conn = in_memory_db();

    let catalog_id =
        kathaloq_lib::db::insert_catalog(&conn, "Test", "/tmp", "2024-01-01T00:00:00Z").unwrap();

    kathaloq_lib::db::insert_file_entry(&conn, catalog_id, None, "a.txt", "a.txt", false, 100, None, Some("txt")).unwrap();
    kathaloq_lib::db::insert_file_entry(&conn, catalog_id, None, "b.txt", "b.txt", false, 200, None, Some("txt")).unwrap();

    let children = kathaloq_lib::db::get_children(&conn, catalog_id, None).unwrap();
    assert_eq!(children.len(), 2);

    kathaloq_lib::db::delete_catalog(&conn, catalog_id).unwrap();

    let catalogs = kathaloq_lib::db::list_catalogs(&conn).unwrap();
    assert_eq!(catalogs.len(), 0);

    // file_entries should be cascade-deleted
    let children = kathaloq_lib::db::get_children(&conn, catalog_id, None).unwrap();
    assert_eq!(children.len(), 0);
}

#[test]
fn test_media_tags_crud() {
    let conn = in_memory_db();

    let catalog_id =
        kathaloq_lib::db::insert_catalog(&conn, "Test", "/tmp", "2024-01-01T00:00:00Z").unwrap();

    let entry_id = kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, None, "song.mp3", "song.mp3", false, 5000, None, Some("mp3"),
    ).unwrap();

    // No tags initially
    let tags = kathaloq_lib::db::get_media_tags(&conn, entry_id).unwrap();
    assert!(tags.is_none());

    // Insert tags — exhaustive every-column roundtrip (the inline roundtrip test
    // only asserts a subset; this is the full serialization oracle).
    kathaloq_lib::db::insert_media_tags(
        &conn, entry_id, Some(180.5), Some(320), Some(44100), Some(2),
        Some("Test Song"), Some("Test Artist"), Some("Test Album"),
        Some("Rock"), Some(2024), Some(1),
    ).unwrap();

    let tags = kathaloq_lib::db::get_media_tags(&conn, entry_id).unwrap().unwrap();
    assert_eq!(tags.title.as_deref(), Some("Test Song"));
    assert_eq!(tags.artist.as_deref(), Some("Test Artist"));
    assert_eq!(tags.album.as_deref(), Some("Test Album"));
    assert_eq!(tags.genre.as_deref(), Some("Rock"));
    assert_eq!(tags.year, Some(2024));
    assert_eq!(tags.track_number, Some(1));
    assert!((tags.duration_secs.unwrap() - 180.5).abs() < 0.01);
    assert_eq!(tags.bitrate, Some(320));
    assert_eq!(tags.sample_rate, Some(44100));
    assert_eq!(tags.channels, Some(2));
}

#[test]
fn test_media_tags_cascade_on_file_delete() {
    let conn = in_memory_db();

    let catalog_id =
        kathaloq_lib::db::insert_catalog(&conn, "Test", "/tmp", "2024-01-01T00:00:00Z").unwrap();

    let entry_id = kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, None, "song.mp3", "song.mp3", false, 5000, None, Some("mp3"),
    ).unwrap();

    kathaloq_lib::db::insert_media_tags(
        &conn, entry_id, Some(180.0), Some(320), None, None,
        None, None, None, None, None, None,
    ).unwrap();

    let tags = kathaloq_lib::db::get_media_tags(&conn, entry_id).unwrap();
    assert!(tags.is_some());

    // Delete file entry — media_tags should cascade
    kathaloq_lib::db::delete_file_entries_by_ids(&conn, &[entry_id]).unwrap();

    let tags = kathaloq_lib::db::get_media_tags(&conn, entry_id).unwrap();
    assert!(tags.is_none());
}

#[test]
fn test_update_file_entry_metadata() {
    let conn = in_memory_db();

    let catalog_id =
        kathaloq_lib::db::insert_catalog(&conn, "Test", "/tmp", "2024-01-01T00:00:00Z").unwrap();

    let entry_id = kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, None, "a.txt", "a.txt", false, 100, Some("2024-01-01T00:00:00Z"), Some("txt"),
    ).unwrap();

    kathaloq_lib::db::update_file_entry_metadata(&conn, entry_id, 200, Some("2024-06-01T00:00:00Z")).unwrap();

    let children = kathaloq_lib::db::get_children(&conn, catalog_id, None).unwrap();
    assert_eq!(children[0].size, 200);
    assert_eq!(children[0].modified.as_deref(), Some("2024-06-01T00:00:00Z"));
}
