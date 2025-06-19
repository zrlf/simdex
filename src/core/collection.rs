use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn find_collections(root: &Path) -> Vec<(PathBuf, String)> {
    let prefix = ".bamboost-collection-";
    WalkDir::new(root)
        .min_depth(1)
        .max_depth(5) // Change as needed
        .into_iter()
        .filter_map(|entry_result| {
            let entry = match entry_result {
                Ok(e) => e,
                Err(err) => {
                    eprintln!("Error reading directory entry: {}", err);
                    return None;
                }
            };

            if !entry.file_type().is_file() {
                return None;
            }

            let name = entry.file_name().to_str()?;
            let uid = name.strip_prefix(prefix)?;
            let parent = entry.path().parent()?;

            Some((parent.to_path_buf(), uid.to_string()))
        })
        .collect()
}

pub fn find_entries(collection_path: &Path) -> Vec<PathBuf> {
    let entries = match fs::read_dir(collection_path) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!(
                "Error reading directory '{}': {}",
                collection_path.display(),
                err
            );
            return Vec::new();
        }
    };

    entries
        .filter_map(|entry| match entry {
            Ok(e) => Some(e),
            Err(err) => {
                eprintln!(
                    "Error reading entry in '{}': {}",
                    collection_path.display(),
                    err
                );
                None
            }
        })
        .filter_map(|e| match e.file_type() {
            Ok(ft) if ft.is_dir() => Some(e),
            Ok(_) => None,
            Err(err) => {
                eprintln!(
                    "Error getting file type for '{}': {}",
                    e.path().display(),
                    err
                );
                None
            }
        })
        .filter(|e| e.path().join("data.h5").exists())
        .map(|e| e.path())
        .collect()
}
