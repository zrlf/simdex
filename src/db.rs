use chrono::DateTime;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

use crate::types::{MetaData, Parameters};

pub fn open_or_init<P: AsRef<Path>>(db_path: P) -> rusqlite::Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS collections (
            uid TEXT PRIMARY KEY,
            path TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS simulations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            collection_uid TEXT NOT NULL,
            path TEXT NOT NULL,
            created_at TEXT,
            description TEXT,
            status TEXT,
            submitted INTEGER,
            parameters_json JSON,
            _last_sync_time TEXT,
            UNIQUE(collection_uid, path)
        );
    "#,
    )?;
    Ok(conn)
}

/// Returns simulation id (rowid)
pub fn upsert_collection(conn: &Connection, uid: &str, path: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO collections (uid, path) VALUES (?1, ?2)",
        params![uid, path],
    )?;
    Ok(())
}

pub fn get_sim_sync_time(
    conn: &Connection,
    collection_uid: &str,
    path: &str,
) -> rusqlite::Result<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT _last_sync_time FROM simulations WHERE collection_uid = ?1 AND path = ?2",
    )?;
    stmt.query_row(params![collection_uid, path], |row| row.get(0))
        .optional()
}

pub fn upsert_simulation(
    conn: &Connection,
    collection_uid: &str,
    path: &str,
    meta: &MetaData,
    parameters: &Parameters,
) -> rusqlite::Result<i64> {
    let parameters_json = serde_json::to_string(parameters).unwrap_or("{}".to_string());

    conn.execute(
        "INSERT INTO simulations (collection_uid, path, created_at, description, status, submitted, parameters_json, _last_sync_time)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(collection_uid, path) DO UPDATE SET
            created_at = excluded.created_at,
            description = excluded.description,
            status = excluded.status,
            submitted = excluded.submitted,
            parameters_json = excluded.parameters_json,
            _last_sync_time = excluded._last_sync_time
        ",
        params![
            collection_uid,
            path,
            meta.created_at.to_rfc3339(),
            meta.description,
            meta.status.as_str(),
            meta.submitted as i32,
            parameters_json,
            chrono::offset::Local::now().to_rfc3339(),
        ],
    )?;

    // get simulation row id
    let mut stmt =
        conn.prepare("SELECT id FROM simulations WHERE collection_uid = ?1 AND path = ?2")?;
    let id: i64 = stmt.query_row(params![collection_uid, path], |row| row.get(0))?;
    Ok(id)
}
