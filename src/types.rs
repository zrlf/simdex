use chrono::{DateTime, Utc};
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
