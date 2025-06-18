mod collection;
mod db;
mod entry;
mod types;

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::{env, fs};

#[derive(Parser)]
#[command(name = "simdex")]
#[command(about = "A tool to manage scientific data", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan & sync simulation data into the cache database
    Sync {
        #[arg(default_value = ".")]
        root: PathBuf,
        #[arg(short, long, default_value = "simdex.db")]
        db: PathBuf,
    },

    LsCollections {
        #[arg(short, long, default_value = "simdex.db")]
        db: PathBuf,
    },

    LsParams {
        #[arg(short, long)]
        db: PathBuf,
        #[arg()]
        collection: String,
    },

    Migrate {
        #[arg(default_value = ".")]
        root: PathBuf,
    },
}

/// Returns the modification time of `data.h5` in RFC3339 format, or None if unavailable.
/// If the file does not exist or cannot be accessed, it returns None.
///
/// # Arguments
/// * `path` - The path to the collection directory containing `data.h5`.
fn get_data_h5_mtime(path: &Path) -> Option<String> {
    let h5_path = path.join("data.h5");
    let meta = fs::metadata(h5_path).ok()?;
    let mtime = meta.modified().ok()?;
    let dt: chrono::DateTime<chrono::Utc> = mtime.into();
    Some(dt.to_rfc3339())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let root = if args.len() > 1 { &args[1] } else { "." };
    let db_path = "simdex.db";

    let mut conn = db::open_or_init(db_path).expect("failed to open SQLite database");

    let collections = collection::find_collections(Path::new(root));
    println!("Found {} collections:", collections.len());

    let tx = conn.transaction().unwrap();

    for (c_path, c_uid) in &collections {
        println!("Collection {}: {:?}", c_uid, c_path);
        db::upsert_collection(&tx, c_uid, &c_path.display().to_string()).expect("db err");
        let entries = collection::find_entries(c_path);

        for entry in entries {
            let entry_path = entry.display().to_string();
            let mtime = match get_data_h5_mtime(&entry) {
                Some(ut) => ut,
                None => {
                    println!("  [!] Failed to get mtime for entry: {:?}", entry);
                    continue;
                }
            };

            // check last sync time in db
            let last_sync_time = db::get_sim_sync_time(&tx, c_uid, &entry_path).unwrap_or(None);

            // only process if changed or new
            if Some(mtime.clone()) <= last_sync_time {
                // unchanged -> skip
                continue;
            }

            match entry::load_entry_meta(&entry) {
                Some((meta, params)) => {
                    let entry_path = entry.display().to_string();
                    let sim_id = db::upsert_simulation(&tx, c_uid, &entry_path, &meta, &params)
                        .expect("db insert sim");
                    println!("  Synced entry: {:?} [{}]", entry, sim_id);
                }
                None => {
                    println!("  [!] Failed to read entry: {:?}", entry);
                }
            }
        }
    }
    tx.commit().ok();

    println!(" Sync complete.");
}
