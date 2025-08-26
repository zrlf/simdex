use polars::prelude::*;

macro_rules! struct_to_dataframe {
    ($input:expr, [$($field:ident),+]) => {
        {
            let len = $input.len().to_owned();

            // Extract the field values into separate vectors
            $(let mut $field = Vec::with_capacity(len);)*

            for e in $input.into_iter() {
                $($field.push(e.$field);)*
            }
            df! {
                $(stringify!($field) => $field,)*
            }
        }
    };
}

pub fn display_polars(uid: &str) {
    let conn = db::open_or_init(config::DEFAULT_DB_PATH).expect("failed to open DB");

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

    // Flatten the parameters field into separate columns
    let (all_keys, columns) = flatten_hashmap_field(&rows, |r| &r.parameters);

    // Prepare vectors for the other fields
    let mut ids = Vec::with_capacity(rows.len());
    let mut names = Vec::with_capacity(rows.len());
    let mut created_ats = Vec::with_capacity(rows.len());
    let mut statuses = Vec::with_capacity(rows.len());
    let mut submitteds = Vec::with_capacity(rows.len());

    for row in &rows {
        ids.push(row.id);
        names.push(row.name.clone());
        created_ats.push(row.created_at.clone());
        statuses.push(row.status.clone());
        submitteds.push(row.submitted);
    }

    // Build the DataFrame with flattened columns
    let mut df_builder = df![
        "id" => ids,
        "name" => names,
        "created_at" => created_ats,
        "status" => statuses,
        "submitted" => submitteds
    ]
    .unwrap();

    for key in &all_keys {
        let col_name = format!("parameters_{}", key);
        let col_values: Vec<Option<String>> = columns.get(key).unwrap().clone();
        let s = Series::new(col_name.into(), col_values);
        df_builder.with_column(s).unwrap();
    }

    println!("{:?}", df_builder);
}
