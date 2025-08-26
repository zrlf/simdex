use pyo3::prelude::*;
use serde_json::Value as JsonValue;
use std::path::Path;
use tabled::{
    Tabled,
    settings::{Color, Style, object::Rows},
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

impl Row {
    fn new(
        id: i64,
        name: String,
        created_at: String,
        status: String,
        submitted: bool,
        parameters_json: String,
    ) -> Self {
        let parsed: JsonValue = serde_json::from_str(&parameters_json).unwrap_or_default();
        let parameters = parsed
            .as_object()
            .unwrap_or(&serde_json::Map::new())
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect();

        Self {
            id,
            name,
            created_at,
            status,
            submitted,
            parameters,
        }
    }
}

/// Flattens a vector of structs with a HashMap field into separate columns for each key in the HashMap.
/// Returns a tuple of (Vec of field vectors, BTreeSet of all keys, Vec of HashMap values per key).
fn flatten_hashmap_field(
    rows: &[Row],
    hashmap_field: fn(&Row) -> &std::collections::HashMap<String, String>,
) -> (
    std::collections::BTreeSet<String>,
    std::collections::HashMap<String, Vec<Option<String>>>,
) {
    let mut all_keys = std::collections::BTreeSet::new();
    for row in rows {
        all_keys.extend(hashmap_field(row).keys().cloned());
    }
    let mut columns: std::collections::HashMap<String, Vec<Option<String>>> =
        std::collections::HashMap::new();
    for key in &all_keys {
        columns.insert(key.clone(), Vec::with_capacity(rows.len()));
    }
    for row in rows {
        let map = hashmap_field(row);
        for key in &all_keys {
            columns.get_mut(key).unwrap().push(map.get(key).cloned());
        }
    }
    (all_keys, columns)
}


pub fn display(db_path: &Path, uid: &str) {
    let conn = db::open_or_init(db_path).expect("failed to open DB");
    let mut stmt = conn
        .prepare(
            "SELECT id, name, created_at, status, submitted, parameters_json
             FROM simulations WHERE collection_uid = ?1",
        )
        .unwrap();
    let rows: Vec<Row> = stmt
        .query_map([uid], |row| {
            Ok(Row::new(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    let (all_keys, _columns) = flatten_hashmap_field(&rows, |r| &r.parameters);

    use tabled::builder::Builder;

    let mut builder = Builder::default();
    let mut header = vec!["id", "status", "submitted", "created_at", "name"];
    header.extend(all_keys.iter().map(|k| k.as_str()));
    builder.push_record(header);

    for row in rows {
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

#[pyfunction]
fn py_display(db_path: &str, collection: &str) -> PyResult<String> {
    let path = Path::new(db_path);
    display(path, collection);
    Ok("Display complete.".to_string())
}

#[pymodule]
#[pyo3(name = "_simdex")]
fn python_module(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_display, m)?)?;
    Ok(())
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
