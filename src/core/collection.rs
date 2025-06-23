use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::core::db;

/// Finds entry directories within a collection directory that contain a "data.h5" file.
///
/// # Arguments
///
/// * `collection_path` - The path to the collection directory to search.
///
/// # Returns
///
/// A vector of `PathBuf` objects, each representing a directory inside the collection
/// that contains a "data.h5" file. Any errors encountered while reading the directory
/// or its entries are printed to stderr, and those entries are skipped.
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

/*
pub fn sync(collection_path: &Path) -> Result<(), String> {
    let entries = find_entries(collection_path);

    for entry in entries {
        let entry_name = entry
            .file_name()
            .expect("entry has no file name")
            .to_string_lossy()
            .to_string();

        // check last sync time in db
        let last_sync_time = db::get_sim_sync_time(&tx, c_uid, &entry_name);

        // only process if changed or new
        let mtime = match crate::core::entry::get_data_h5_mtime(&entry) {
            Some(ut) => ut,
            None => {
                eprintln!("  [!] Failed to get mtime for entry: {:?}", entry);
                continue;
            }
        };

        // if last_sync_time is None, this will be false (not skipped)
        if Some(mtime) < last_sync_time {
            // unchanged -> skip
            continue;
        }

        match entry::load_entry_meta(&entry) {
            Some((meta, params)) => {
                let sim_id = db::upsert_simulation(&tx, c_uid, &entry_name, &meta, &params)
                    .expect("db insert sim");
                println!("  Synced entry: {:?} [{}]", entry, sim_id);
            }
            None => {
                println!("  [!] Failed to read entry: {:?}", entry);
            }
        }
    }
    Ok(())
}
*/
