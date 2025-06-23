use serde_json::Value as JsonValue;
use std::{fs, path::Path};
use tabled::{
    Tabled,
    settings::{Color, Style, formatting::Justification, object::Rows, themes::Colorization},
};

use crate::core::{collection, db, discovery, entry};

#[derive(Tabled)]
struct Row {
    id: i64,
    name: String,
    created_at: String,
    status: String,
    submitted: bool,
    #[tabled(skip)]
    parameters: std::collections::HashMap<String, String>,
}

pub fn display(db_path: &Path, collection: &str) {
    let conn = db::open_or_init(db_path).expect("failed to open DB");

    let mut stmt = conn
        .prepare(
            "SELECT id, name, created_at, status, submitted, parameters_json
         FROM simulations WHERE collection_uid = ?1",
        )
        .unwrap();

    let rows = stmt
        .query_map([collection], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let created_at: String = (row.get(2)?);
            let status: String = row.get(3)?;
            let submitted: bool = row.get(4)?;
            let json: String = row.get(5)?;
            let parsed: JsonValue = serde_json::from_str(&json).unwrap_or_default();

            let params = parsed
                .as_object()
                .unwrap_or(&serde_json::Map::new())
                .iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect();

            Ok(Row {
                id,
                name,
                created_at,
                status,
                submitted,
                parameters: params,
            })
        })
        .unwrap();

    let mut table_rows = vec![];
    let mut all_keys = std::collections::BTreeSet::new();

    for row in rows {
        let r = row.unwrap();
        all_keys.extend(r.parameters.keys().cloned());
        table_rows.push(r);
    }

    use tabled::builder::Builder;

    let mut builder = Builder::default();
    let mut header = vec!["id", "status", "submitted", "created_at", "name"];
    header.extend(all_keys.iter().map(|k| k.as_str()));
    builder.push_record(header);

    for row in &table_rows {
        let mut values = vec![
            row.id.to_string(),
            row.status.clone(),
            row.submitted.to_string(),
            row.created_at.clone(),
            row.name.clone(),
        ];
        for key in &all_keys {
            values.push(row.parameters.get(key).cloned().unwrap_or_default());
        }
        builder.push_record(values);
    }

    let mut table = builder.build();
    table.with(Style::blank());
    table.modify(Rows::first(), Color::FG_BRIGHT_BLACK);
    println!("{}", table);
}

pub fn scan(root: &Path, db_path: &Path) {
    let mut conn = db::open_or_init(db_path).expect("failed to open SQLite database");

    let collections = discovery::find_all(Path::new(root));
    println!("Found {} collections:", collections.len());

    let tx = conn.transaction().unwrap();

    for (c_path, c_uid) in &collections {
        println!("Collection {}: {:?}", c_uid, c_path);
        db::upsert_collection(&tx, c_uid, &c_path.display().to_string()).expect("db err");
        let entries = collection::find_entries(c_path);

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
    }
    tx.commit().ok();

    println!(" Sync complete.");
}

pub fn ls_collections(db_path: &Path) {
    let conn = db::open_or_init(db_path).expect("failed to open DB");
    let mut stmt = conn.prepare("SELECT uid, path FROM collections").unwrap();
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap();

    println!("Collections:");
    for row in rows {
        let (uid, path) = row.unwrap();
        println!(" - {} @ {}", uid, path);
    }
}

pub fn ls_params(db_path: &Path, collection: &str) {
    let conn = db::open_or_init(db_path).expect("failed to open DB");
    let mut stmt = conn
        .prepare("SELECT parameters_json FROM simulations WHERE collection_uid = ?1")
        .unwrap();
    let mut rows = stmt.query([collection]).unwrap();

    let mut all_keys = std::collections::HashSet::new();
    let mut examples = std::collections::HashMap::new();

    while let Some(row) = rows.next().unwrap() {
        let json: String = row.get(0).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
        if let Some(obj) = parsed.as_object() {
            for (k, v) in obj {
                all_keys.insert(k.clone());
                examples.entry(k.clone()).or_insert_with(|| v.to_string());
            }
        }
    }

    println!("Parameter space of '{}':", collection);
    for key in all_keys {
        let placeholder = "<none>".to_string();
        let example = examples.get(&key).unwrap_or(&placeholder);
        println!(" - {:20} e.g. {}", key, example);
    }
}

pub fn migrate(root: &Path) {
    use crate::core::entry::load_entry_meta;
    use std::fs::write;

    let collections = discovery::find_all(root);
    for (c_path, _) in &collections {
        let entries = collection::find_entries(c_path);
        for entry in entries {
            if let Some((meta, params)) = load_entry_meta(&entry) {
                let yaml_out = serde_yaml::to_string(&serde_json::json!({
                    "metadata": {
                        "created_at": meta.created_at.to_rfc3339(),
                        "description": meta.description,
                        "status": meta.status,
                        "submitted": meta.submitted,
                    },
                    "parameters": params
                }))
                .unwrap();
                let out_path = entry.join("meta.yml");
                write(out_path, yaml_out).expect("write failed");
                println!("Migrated {:?}", entry);
            }
        }
    }
}
