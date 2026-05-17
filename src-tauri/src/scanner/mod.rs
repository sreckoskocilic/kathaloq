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
