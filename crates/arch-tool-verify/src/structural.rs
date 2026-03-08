use std::path::Path;

/// Check if a file exists relative to repo root.
pub fn file_exists(repo_root: &Path, path: &str) -> bool {
    repo_root.join(path).is_file()
}

/// Check if a directory exists relative to repo root.
pub fn dir_exists(repo_root: &Path, path: &str) -> bool {
    repo_root.join(path).is_dir()
}
