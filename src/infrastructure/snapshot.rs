//! The offline snapshot directory resolver (ADR-0003 §1): total, platform-
//! aware, and distinct from the *config* dir (the user's selection is not
//! derived cache data).

use std::path::PathBuf;

/// `$cache_dir/mawaqit-tui/snapshots`, degrading to the temp dir when the
/// platform has no cache dir. Total: always returns a directory path.
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("mawaqit-tui")
        .join("snapshots")
}

#[cfg(test)]
mod tests {
    #[test]
    fn cache_dir_is_total_and_scoped() {
        let dir = super::cache_dir();
        assert!(dir.starts_with(dirs::cache_dir().unwrap_or(std::env::temp_dir())));
        assert!(dir.ends_with("mawaqit-tui/snapshots"));
    }
}
