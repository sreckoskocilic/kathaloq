pub mod media;
mod updater;
mod walker;

pub use updater::{apply_update, preview_update};
pub use walker::scan_directory;

const IGNORED_NAMES: &[&str] = &[
    "$RECYCLE.BIN",
    "System Volume Information",
    "$WinREAgent",
    "RECYCLER",
    "Thumbs.db",
    "desktop.ini",
    "ehthumbs.db",
];

pub fn should_skip(name: &str) -> bool {
    name.starts_with("._")
        || name.starts_with('.')
        || IGNORED_NAMES.iter().any(|&n| n.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::should_skip;

    #[test]
    fn skips_hidden_and_system_files() {
        assert!(should_skip(".hidden"));
        assert!(should_skip("._resourcefork"));
        assert!(should_skip("Thumbs.db"));
        assert!(should_skip("thumbs.db")); // case-insensitive
        assert!(should_skip("desktop.ini"));
        assert!(should_skip("System Volume Information"));
    }

    #[test]
    fn keeps_normal_names() {
        assert!(!should_skip("song.mp3"));
        assert!(!should_skip("Music"));
        assert!(!should_skip("holiday_2026.mp4"));
    }
}
