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
            _last_sync_time TEXT,
            UNIQUE(collection_uid, path)
        );
        CREATE TABLE IF NOT EXISTS parameters (
            id INTEGER PRIMARY KEY,
            simulation_id INTEGER NOT NULL,
            key TEXT NOT NULL,
            value TEXT NOT NULL,
            UNIQUE(simulation_id, key)
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
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO simulations (collection_uid, path, created_at, description, status, submitted, _last_sync_time)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(collection_uid, path) DO UPDATE SET
            created_at = excluded.created_at,
            description = excluded.description,
            status = excluded.status,
            submitted = excluded.submitted,
            _last_sync_time = excluded._last_sync_time
        ",
        params![
            collection_uid,
            path,
            meta.created_at.to_rfc3339(),
            meta.description,
            meta.status.as_str(),
            meta.submitted as i32,
            chrono::offset::Local::now().to_rfc3339(),
        ],
    )?;

    // get simulation row id
    let mut stmt =
        conn.prepare("SELECT id FROM simulations WHERE collection_uid = ?1 AND path = ?2")?;
    let id: i64 = stmt.query_row(params![collection_uid, path], |row| row.get(0))?;
    Ok(id)
}

pub fn set_parameters(
    conn: &Connection,
    simulation_id: i64,
    parameters: &Parameters,
) -> rusqlite::Result<()> {
    // remove old parameters for this simulation
    conn.execute(
        "DELETE FROM parameters WHERE simulation_id=?1",
        params![simulation_id],
    )?;
    for (k, v) in parameters {
        conn.execute(
            "INSERT INTO parameters (simulation_id, key, value) VALUES (?1, ?2, ?3)",
            params![simulation_id, k, v.to_string()],
        )?;
    }
    Ok(())
}
