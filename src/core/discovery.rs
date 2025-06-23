use crate::config;
use crate::core::db;
use crate::core::types::{Author, MetaFile};
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::{fs, io};
use walkdir::WalkDir;

fn get_author() -> Option<Author> {
    fn _git_user() -> Option<Author> {
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

    fn _system_user() -> Option<Author> {
        None
    }

    _git_user().or_else(_system_user)
}

fn create_identifier(path: &Path, uid: &str) -> std::io::Result<()> {
    let timestamp = chrono::Local::now().to_rfc3339();
    let meta_file = path
        .join(format!("{}{}", config::META_FILE_PREFIX, uid))
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

pub fn new_collection(path: impl Into<PathBuf>, uid: &str) -> std::io::Result<()> {
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
pub fn find_all(root: &Path) -> Vec<(PathBuf, String)> {
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
            let uid_raw = name.strip_prefix(config::META_FILE_PREFIX)?;
            let uid = uid_raw.strip_suffix(".yml").unwrap_or(uid_raw);
            let parent = entry.path().parent()?;

            Some((parent.to_path_buf(), uid.to_string()))
        })
        .collect()
}

fn find_one(uid: &str, root: Option<&Path>) -> io::Result<PathBuf> {
    let root = root.unwrap_or_else(|| Path::new("."));
    let patterns = [
        format!("{}{}", config::META_FILE_PREFIX, uid),
        format!("{}{}.yml", config::META_FILE_PREFIX, uid),
    ];

    for entry in WalkDir::new(root)
        .min_depth(1)
        .max_depth(5)
        .into_iter()
        .flatten()
    {
        if entry.file_type().is_file() {
            let file_name = entry.file_name().to_string_lossy();
            if patterns.iter().any(|p| p == &file_name) {
                return Ok(entry.path().parent().map(|p| p.to_path_buf()).unwrap());
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Collection with UID '{}' not found", uid),
    ))
}

pub fn get_path(uid: &str) -> io::Result<PathBuf> {
    let conn = db::open_or_init(config::DEFAULT_DB_PATH).expect("Failed to open DB");

    match db::get_collection_path(&conn, uid).filter(|p| p.exists()) {
        Some(path) => Ok(path),
        None => find_one(uid, None),
    }
}

fn read_uid_from_meta_file(path: &Path) -> Result<String, String> {
    use regex::Regex;

    // Regex: ^\.bamboost-collection-(?P<uid>[^\.]+)(\.yml)?$
    let re = Regex::new(&format!(
        r"^{}(?P<uid>[^\.]+)(\.yml)?$",
        regex::escape(config::META_FILE_PREFIX)
    ))
    .map_err(|e| format!("Failed to compile regex: {}", e))?;

    let entries = fs::read_dir(path)
        .map_err(|err| format!("Failed to read directory '{}': {}", path.display(), err))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                eprintln!("Error reading directory entry: {}", err);
                continue;
            }
        };
        let file_name = entry.file_name();
        let file_name_str = match file_name.to_str() {
            Some(s) => s,
            None => continue,
        };
        if let Some(caps) = re.captures(file_name_str) {
            if let Some(uid) = caps.name("uid") {
                return Ok(uid.as_str().to_string());
            }
        }
    }

    Err(format!(
        "No collection file found in '{}'. Expected a file starting with '{}'",
        path.display(),
        config::META_FILE_PREFIX
    ))
}

pub fn get_uid(path: &Path) -> Result<String, String> {
    if !path.exists() || !path.is_dir() {
        return Err(format!("Path '{}' is not a directory", path.display()));
    }
    read_uid_from_meta_file(path)
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
            .join(format!("{}{}", config::META_FILE_PREFIX, uid))
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
