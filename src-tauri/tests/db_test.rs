use rusqlite::Connection;

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
fn test_insert_file_entry_with_hierarchy() {
    let conn = in_memory_db();

    let catalog_id =
        kathaloq_lib::db::insert_catalog(&conn, "Test", "/tmp", "2024-01-01T00:00:00Z").unwrap();

    let dir_id = kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, None, "docs", "docs", true, 0, None, None,
    ).unwrap();

    kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, Some(dir_id), "readme.md", "docs/readme.md", false, 1024, None, Some("md"),
    ).unwrap();

    let root = kathaloq_lib::db::get_children(&conn, catalog_id, None).unwrap();
    assert_eq!(root.len(), 1);
    assert!(root[0].is_dir);

    let children = kathaloq_lib::db::get_children(&conn, catalog_id, Some(dir_id)).unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name, "readme.md");
}

#[test]
fn test_search_files() {
    let conn = in_memory_db();

    let catalog_id =
        kathaloq_lib::db::insert_catalog(&conn, "Test", "/tmp", "2024-01-01T00:00:00Z").unwrap();

    kathaloq_lib::db::insert_file_entry(&conn, catalog_id, None, "readme.md", "readme.md", false, 1024, None, Some("md")).unwrap();
    kathaloq_lib::db::insert_file_entry(&conn, catalog_id, None, "main.rs", "main.rs", false, 512, None, Some("rs")).unwrap();

    let results = kathaloq_lib::db::search_files(&conn, catalog_id, "readme").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "readme.md");

    let results = kathaloq_lib::db::search_files(&conn, catalog_id, ".rs").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "main.rs");
}

#[test]
fn test_collect_descendants_and_remove() {
    let conn = in_memory_db();

    let catalog_id =
        kathaloq_lib::db::insert_catalog(&conn, "Test", "/tmp", "2024-01-01T00:00:00Z").unwrap();

    let dir_id = kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, None, "music", "music", true, 0, None, None,
    ).unwrap();

    let sub_id = kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, Some(dir_id), "album", "music/album", true, 0, None, None,
    ).unwrap();

    kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, Some(sub_id), "song.mp3", "music/album/song.mp3", false, 5000, None, Some("mp3"),
    ).unwrap();

    kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, None, "other.txt", "other.txt", false, 100, None, Some("txt"),
    ).unwrap();

    // Collect descendants of music folder
    let descendants = kathaloq_lib::db::collect_descendant_ids(&conn, &[dir_id]).unwrap();
    assert_eq!(descendants.len(), 3); // music, album, song.mp3

    // Delete music folder and all descendants
    kathaloq_lib::db::delete_file_entries_by_ids(&conn, &descendants).unwrap();

    let remaining = kathaloq_lib::db::get_children(&conn, catalog_id, None).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].name, "other.txt");
}

#[test]
fn test_recalc_catalog_stats() {
    let conn = in_memory_db();

    let catalog_id =
        kathaloq_lib::db::insert_catalog(&conn, "Test", "/tmp", "2024-01-01T00:00:00Z").unwrap();

    kathaloq_lib::db::insert_file_entry(&conn, catalog_id, None, "a.txt", "a.txt", false, 100, None, Some("txt")).unwrap();
    kathaloq_lib::db::insert_file_entry(&conn, catalog_id, None, "b.txt", "b.txt", false, 200, None, Some("txt")).unwrap();
    kathaloq_lib::db::insert_file_entry(&conn, catalog_id, None, "dir", "dir", true, 0, None, None).unwrap();

    kathaloq_lib::db::recalc_catalog_stats(&conn, catalog_id).unwrap();

    let cat = kathaloq_lib::db::get_catalog_by_id(&conn, catalog_id).unwrap();
    assert_eq!(cat.total_files, 2);
    assert_eq!(cat.total_size, 300);
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

    // Insert tags
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
fn test_media_tags_bulk() {
    let conn = in_memory_db();

    let catalog_id =
        kathaloq_lib::db::insert_catalog(&conn, "Test", "/tmp", "2024-01-01T00:00:00Z").unwrap();

    let e1 = kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, None, "a.mp3", "a.mp3", false, 1000, None, Some("mp3"),
    ).unwrap();
    let e2 = kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, None, "b.flac", "b.flac", false, 2000, None, Some("flac"),
    ).unwrap();
    let e3 = kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, None, "c.txt", "c.txt", false, 100, None, Some("txt"),
    ).unwrap();

    kathaloq_lib::db::insert_media_tags(&conn, e1, Some(200.0), None, None, None, Some("Song A"), Some("Artist"), None, None, None, None).unwrap();
    kathaloq_lib::db::insert_media_tags(&conn, e2, Some(300.0), None, None, None, Some("Song B"), Some("Artist"), None, None, None, None).unwrap();

    let bulk = kathaloq_lib::db::get_media_tags_bulk(&conn, &[e1, e2, e3]).unwrap();
    assert_eq!(bulk.len(), 2); // c.txt has no tags
    assert_eq!(bulk[0].title.as_deref(), Some("Song A"));
    assert_eq!(bulk[1].title.as_deref(), Some("Song B"));
}

#[test]
fn test_folder_stats() {
    let conn = in_memory_db();

    let catalog_id =
        kathaloq_lib::db::insert_catalog(&conn, "Test", "/tmp", "2024-01-01T00:00:00Z").unwrap();

    let dir_id = kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, None, "music", "music", true, 0, None, None,
    ).unwrap();

    let sub_id = kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, Some(dir_id), "album", "music/album", true, 0, None, None,
    ).unwrap();

    kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, Some(sub_id), "a.mp3", "music/album/a.mp3", false, 3000, None, Some("mp3"),
    ).unwrap();
    kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, Some(sub_id), "b.mp3", "music/album/b.mp3", false, 4000, None, Some("mp3"),
    ).unwrap();
    kathaloq_lib::db::insert_file_entry(
        &conn, catalog_id, Some(dir_id), "c.mp3", "music/c.mp3", false, 2000, None, Some("mp3"),
    ).unwrap();

    let stats = kathaloq_lib::db::get_folder_stats(&conn, catalog_id, dir_id).unwrap();
    assert_eq!(stats.file_count, 3);
    assert_eq!(stats.folder_count, 1); // album subfolder
    assert_eq!(stats.total_size, 9000);
}

#[test]
fn test_is_media_file() {
    use kathaloq_lib::scanner::media::is_media_file;

    assert!(is_media_file(Some("mp3")));
    assert!(is_media_file(Some("MP3")));
    assert!(is_media_file(Some("flac")));
    assert!(is_media_file(Some("m4a")));
    assert!(is_media_file(Some("mp4")));
    assert!(is_media_file(Some("wav")));
    assert!(is_media_file(Some("ogg")));
    assert!(!is_media_file(Some("txt")));
    assert!(!is_media_file(Some("pdf")));
    assert!(!is_media_file(None));
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
