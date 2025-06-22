use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::{fs, io};
use walkdir::WalkDir;

const META_FILE_PREFIX: &str = ".bamboost-collection-";

#[derive(Serialize)]
struct Author {
    name: String,
    email: String,
}

#[derive(Serialize)]
struct MetaFile<'a> {
    uid: &'a str,
    created: &'a str,
    author: Option<Author>,
}

fn git_user() -> Option<Author> {
    let name = Command::new("git")
        .args(["config", "--get", "user.name"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());

    let email = Command::new("git")
        .args(["config", "--get", "user.email"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());

    match (name, email) {
        (Some(name), Some(email)) => Some(Author { name, email }),
        _ => None,
    }
}

fn system_user() -> Option<Author> {
    None
}

fn get_author() -> Option<Author> {
    git_user().or_else(system_user)
}

fn create_identifier(path: &Path, uid: &str) -> std::io::Result<()> {
    let timestamp = chrono::Local::now().to_rfc3339();
    let meta_file = path
        .join(format!("{}{}", META_FILE_PREFIX, uid))
        .with_extension("yml");

    let yaml = serde_yaml::to_string(&MetaFile {
        uid,
        created: &timestamp,
        author: get_author(), // Optionally set the author
    })
    .unwrap_or_else(|_| {
        eprintln!("Failed to serialize metadata to YAML");
        String::new()
    });

    fs::write(&meta_file, yaml)?;

    Ok(())
}

pub fn create_collection(path: impl Into<PathBuf>, uid: &str) -> std::io::Result<()> {
    let path: PathBuf = path.into();
    let _uid: String = uid.into();

    if path.exists() {
        if !path.is_dir() {
            return Err(std::io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "Path '{}' already exists and is not a directory",
                    path.display()
                ),
            ));
        }
        let mut dir = fs::read_dir(&path)?;
        if dir.next().is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::DirectoryNotEmpty,
                format!(
                    "Directory '{}' already exists and is not empty",
                    path.display()
                ),
            ));
        }
        // Directory exists and is empty
    } else {
        fs::create_dir_all(&path)?;
    }

    // Create the identifier file
    create_identifier(&path, uid)?;

    Ok(())
}

/// Reads the collection meta file from the specified directory and extracts the unique identifier (UID).
///
/// The meta file is expected to have a filename starting with the prefix defined by `META_FILE_PREFIX`.
/// This function searches for such a file in the given directory and returns the UID portion of the filename.
///
/// # Arguments
///
/// * `path` - The path to the directory to search for the meta file.
///
/// # Returns
///
/// * `Ok(String)` containing the UID if a meta file is found.
/// * `Err(String)` with an error message if no meta file is found or if the directory cannot be read.
fn read_meta_file(path: &Path) -> Result<String, String> {
    let (_meta_file, uid) = path
        .read_dir()
        .map_err(|err| format!("Failed to read directory '{}': {}", path.display(), err))?
        .find_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                let file_name = path.file_stem()?;
                let file_name_string = file_name.to_string_lossy();
                let uid = file_name_string.strip_prefix(META_FILE_PREFIX)?;
                Some((path.clone(), uid.to_string()))
            })
        })
        .map(|(meta_file, uid)| (Some(meta_file), Some(uid)))
        .unwrap_or((None, None));

    match uid {
        Some(uid) => Ok(uid),
        None => Err(format!(
            "No collection file found in '{}'. Expected a file starting with '{}'",
            path.display(),
            META_FILE_PREFIX
        )),
    }
}

/// Searches for collection files within the given root directory.
///
/// A collection file is identified by its filename starting with the prefix ".bamboost-collection-".
/// The function recursively searches up to 5 levels deep from the root directory.
///
/// # Arguments
///
/// * `root` - The root directory to search for collection files.
///
/// # Returns
///
/// A vector of tuples, where each tuple contains:
/// - The parent directory of the collection file (`PathBuf`)
/// - The unique identifier (`String`) extracted from the filename after the prefix
///
/// # Errors
///
/// Any errors encountered while reading directories or entries are printed to stderr,
/// and those entries are skipped.
pub fn find_collections(root: &Path) -> Vec<(PathBuf, String)> {
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
            let uid_raw = name.strip_prefix(META_FILE_PREFIX)?;
            let uid = uid_raw.strip_suffix(".yml").unwrap_or(uid_raw);
            let parent = entry.path().parent()?;

            Some((parent.to_path_buf(), uid.to_string()))
        })
        .collect()
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;
    use std::fs;

    #[test]
    fn test_create_identifier_creates_yaml_with_timestamp() {
        let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let uid = "testuid";
        let path = tmp_dir.path();

        // Call the function
        create_identifier(path, uid).expect("Failed to create identifier");

        // Check file exists
        let meta_file = path
            .join(format!("{}{}", META_FILE_PREFIX, uid))
            .with_extension("yml");
        assert!(meta_file.exists(), "Meta file was not created");

        // Read and parse YAML
        let contents = fs::read_to_string(&meta_file).expect("Failed to read meta file");
        let yaml: Value = serde_yaml::from_str(&contents).expect("Failed to parse YAML");

        // Check that the 'created' field exists and is a string
        assert!(
            yaml.get("created").is_some(),
            "YAML missing 'created' field"
        );
        assert!(
            yaml["created"].as_str().is_some(),
            "'created' field is not a string"
        );
    }
}
