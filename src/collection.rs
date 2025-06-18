use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn find_collections(root: &Path) -> Vec<(PathBuf, String)> {
    WalkDir::new(root)
        .min_depth(1)
        .max_depth(5) // Change as needed
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            if entry.file_type().is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    let prefix = ".bamboost-collection-";
                    if let Some(uid) = name.strip_prefix(prefix) {
                        return Some((
                            entry.path().parent().unwrap().to_path_buf(),
                            uid.to_string(),
                        ));
                    }
                }
            }
            None
        })
        .collect()
}

pub fn find_entries(collection_path: &Path) -> Vec<PathBuf> {
    fs::read_dir(collection_path)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().ok().map(|ft| ft.is_dir()).unwrap_or(false))
                .filter(|e| e.path().join("data.h5").exists())
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default()
}
