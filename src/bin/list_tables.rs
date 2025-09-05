use anyhow::Result;
use sqlx::Row;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let db_name = args.get(1).map(|s| s.as_str()).unwrap_or("unagi");

    let query = "SELECT table_name FROM information_schema.tables WHERE table_schema = ? ORDER BY table_name";
    let rows = icfpc2025::sql::select1(query, db_name)?;

    for row in rows {
        let table_name: String = row.try_get(0)?;
        println!("\nTable: {}", table_name);

        let schema_query = "SELECT column_name, column_type, is_nullable, column_key, column_default, extra FROM information_schema.columns WHERE table_schema = ? AND table_name = ? ORDER BY ordinal_position";
        let schema_rows = icfpc2025::sql::select2(schema_query, db_name, &table_name)?;

        for schema_row in schema_rows {
            let column_name: String = schema_row.try_get(0)?;
            let column_type: String = schema_row.try_get(1)?;
            let is_nullable: String = schema_row.try_get(2)?;
            let column_key: String = schema_row.try_get(3)?;
            let column_default: Option<String> = schema_row.try_get(4)?;
            let extra: String = schema_row.try_get(5)?;

            let mut details = vec![column_type];
            if is_nullable == "NO" {
                details.push("NOT NULL".to_string());
            }
            if !column_key.is_empty() {
                details.push(format!("KEY: {}", column_key));
            }
            if let Some(default) = column_default {
                details.push(format!("DEFAULT: {}", default));
            }
            if !extra.is_empty() {
                details.push(extra);
            }

            println!("  {}: {}", column_name, details.join(", "));
        }
    }

    Ok(())
}
