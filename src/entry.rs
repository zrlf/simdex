use crate::types::{MetaData, Parameters};
use chrono::{DateTime, Utc};
use hdf5::File;
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

#[derive(Deserialize)]
struct TypeWrapper {
    #[serde(rename = "__type__")]
    _type: String,
    #[serde(rename = "__value__")]
    value: String,
}

fn parse_datetime_field(val: &str) -> Option<DateTime<Utc>> {
    if let Ok(wrapped) = serde_json::from_str::<TypeWrapper>(val) {
        if wrapped._type == "datetime" {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&wrapped.value) {
                return Some(dt.with_timezone(&Utc));
            } else if let Ok(dt) = DateTime::parse_from_rfc3339(&format!("{}Z", wrapped.value)) {
                return Some(dt.with_timezone(&Utc));
            }
        }
    }
    None
}

pub fn load_entry_meta(entry_path: &Path) -> Option<(MetaData, Parameters)> {
    let h5_path = entry_path.join("data.h5");
    let file = File::open(&h5_path).ok()?;
    let root = file.group("/").ok()?;

    // Extract metadata attributes
    let created_at_str: String = root
        .attr("created_at")
        .ok()?
        .read_scalar::<hdf5::types::VarLenUnicode>()
        .ok()?
        .to_string();
    let created_at = match parse_datetime_field(&created_at_str) {
        Some(dt) => dt,
        None => {
            eprintln!("Failed to parse created_at: {}", created_at_str);
            DateTime::from_timestamp_nanos(0)
        }
    };

    let description: String = root
        .attr("description")
        .ok()?
        .read_scalar::<hdf5::types::VarLenUnicode>()
        .ok()?
        .to_string();
    let status: String = root
        .attr("status")
        .ok()?
        .read_scalar::<hdf5::types::VarLenUnicode>()
        .ok()?
        .to_string();
    let submitted: bool = root
        .attr("submitted")
        .and_then(|attr| attr.read_scalar::<bool>())
        .unwrap_or(false);

    let metadata = MetaData {
        created_at,
        description,
        status,
        submitted,
    };

    // Extract parameters
    let params_group = root.group(".parameters").ok()?;
    let mut parameters = Parameters::new();

    for attr_name in params_group.attr_names().ok()? {
        let attr = params_group.attr(&attr_name).ok()?;
        let value = if let Ok(scalar) = attr.read_scalar::<i64>() {
            Value::from(scalar)
        } else if let Ok(scalar) = attr.read_scalar::<f64>() {
            Value::from(scalar)
        } else if let Ok(scalar) = attr.read_scalar::<hdf5::types::VarLenUnicode>() {
            Value::from(scalar.to_string())
        } else {
            continue; // Skip unsupported types
        };
        parameters.insert(attr_name, value);
    }

    Some((metadata, parameters))
}
