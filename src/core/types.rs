use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug)]
pub struct MetaData {
    pub created_at: DateTime<Utc>,
    pub description: String,
    pub status: String,
    pub submitted: bool,
}

pub type Parameters = HashMap<String, Value>;

#[derive(Serialize)]
pub struct Author {
    pub name: String,
    pub email: String,
}

#[derive(Serialize)]
pub struct MetaFile<'a> {
    pub uid: &'a str,
    pub created: &'a str,
    pub author: Option<Author>,
}
