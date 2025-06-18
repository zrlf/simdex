mod collection;
mod db;
mod entry;
mod types;

use std::path::Path;
use std::{env, fs};

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

    for (c_path, c_uid) in &collections {
        println!("Collection: {:?}", c_uid);
        db::upsert_collection(&conn, c_uid, &c_path.display().to_string()).expect("db err");
        let entries = collection::find_entries(c_path);

        let tx = conn.transaction().unwrap();
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
                    let sim_id = db::upsert_simulation(&tx, c_uid, &entry_path, &meta)
                        .expect("db insert sim");
                    db::set_parameters(&tx, sim_id, &params).expect("db insert params");
                    println!("  Synced entry: {:?}", entry);
                }
                None => {
                    println!("  [!] Failed to read entry: {:?}", entry);
                }
            }
        }
        tx.commit().unwrap();
    }

    println!("Sync complete.");
}
