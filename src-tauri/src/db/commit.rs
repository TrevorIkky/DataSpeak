use crate::db::connection::{ConnectionManager, DatabaseType};
use crate::error::AppResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellEdit {
    pub row_index: usize,
    pub column_name: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowInsert {
    pub temp_id: String,
    pub row_data: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataGridChanges {
    pub edits: Vec<CellEdit>,
    pub deletes: Vec<usize>, // Row indices
    pub inserts: Vec<RowInsert>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRequest {
    pub connection_id: String,
    pub table_name: String,
    pub primary_key_columns: Vec<String>,
    pub changes: DataGridChanges,
    pub original_rows: Vec<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResult {
    pub success: bool,
    pub message: String,
    pub edits_count: usize,
    pub deletes_count: usize,
    pub inserts_count: usize,
}

pub async fn commit_data_changes(
    manager: &ConnectionManager,
    request: CommitRequest,
) -> AppResult<CommitResult> {
    let conn = manager.get_connection(&request.connection_id)?;

    match conn.database_type {
        DatabaseType::PostgreSQL => commit_postgres_changes(manager, request).await,
        DatabaseType::MariaDB | DatabaseType::MySQL => commit_mysql_changes(manager, request).await,
    }
}

async fn commit_postgres_changes(
    manager: &ConnectionManager,
    request: CommitRequest,
) -> AppResult<CommitResult> {
    let pool = manager.get_pool_postgres(&request.connection_id).await?;
    let mut tx = pool.begin().await?;

    let mut edits_count = 0;
    let mut deletes_count = 0;
    let mut inserts_count = 0;

    // Process deletes first
    for row_index in &request.changes.deletes {
        if let Some(row_data) = request.original_rows.get(*row_index) {
            let where_clause = build_where_clause_postgres(&request.primary_key_columns, row_data);
            let delete_query = format!(
                "DELETE FROM {} WHERE {}",
                quote_identifier_postgres(&request.table_name),
                where_clause
            );

            sqlx::query(&delete_query).execute(&mut *tx).await?;
            deletes_count += 1;
        }
    }

    // Process edits (group by row)
    let mut edits_by_row: std::collections::HashMap<usize, Vec<&CellEdit>> =
        std::collections::HashMap::new();

    for edit in &request.changes.edits {
        edits_by_row.entry(edit.row_index).or_default().push(edit);
    }

    for (row_index, row_edits) in edits_by_row {
        if let Some(row_data) = request.original_rows.get(row_index) {
            let set_clause = row_edits
                .iter()
                .map(|edit| {
                    let value_str = json_value_to_sql_string_postgres(&edit.new_value);
                    format!("{} = {}", quote_identifier_postgres(&edit.column_name), value_str)
                })
                .collect::<Vec<_>>()
                .join(", ");

            let where_clause = build_where_clause_postgres(&request.primary_key_columns, row_data);

            let update_query = format!(
                "UPDATE {} SET {} WHERE {}",
                quote_identifier_postgres(&request.table_name),
                set_clause,
                where_clause
            );

            sqlx::query(&update_query).execute(&mut *tx).await?;
            edits_count += row_edits.len();
        }
    }

    // Process inserts
    for insert in &request.changes.inserts {
        let columns: Vec<String> = insert.row_data.keys()
            .map(|k| quote_identifier_postgres(k))
            .collect();

        let values: Vec<String> = insert.row_data.values()
            .map(json_value_to_sql_string_postgres)
            .collect();

        let insert_query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            quote_identifier_postgres(&request.table_name),
            columns.join(", "),
            values.join(", ")
        );

        sqlx::query(&insert_query).execute(&mut *tx).await?;
        inserts_count += 1;
    }

    tx.commit().await?;

    Ok(CommitResult {
        success: true,
        message: format!(
            "Successfully committed {} edits, {} deletes, {} inserts",
            edits_count, deletes_count, inserts_count
        ),
        edits_count,
        deletes_count,
        inserts_count,
    })
}

async fn commit_mysql_changes(
    manager: &ConnectionManager,
    request: CommitRequest,
) -> AppResult<CommitResult> {
    let pool = manager.get_pool_mysql(&request.connection_id).await?;
    let mut tx = pool.begin().await?;

    let mut edits_count = 0;
    let mut deletes_count = 0;
    let mut inserts_count = 0;

    // Process deletes first
    for row_index in &request.changes.deletes {
        if let Some(row_data) = request.original_rows.get(*row_index) {
            let where_clause = build_where_clause_mysql(&request.primary_key_columns, row_data);
            let delete_query = format!(
                "DELETE FROM {} WHERE {}",
                quote_identifier_mysql(&request.table_name),
                where_clause
            );

            sqlx::query(&delete_query).execute(&mut *tx).await?;
            deletes_count += 1;
        }
    }

    // Process edits (group by row)
    let mut edits_by_row: std::collections::HashMap<usize, Vec<&CellEdit>> =
        std::collections::HashMap::new();

    for edit in &request.changes.edits {
        edits_by_row.entry(edit.row_index).or_default().push(edit);
    }

    for (row_index, row_edits) in edits_by_row {
        if let Some(row_data) = request.original_rows.get(row_index) {
            let set_clause = row_edits
                .iter()
                .map(|edit| {
                    let value_str = json_value_to_sql_string_mysql(&edit.new_value);
                    format!("{} = {}", quote_identifier_mysql(&edit.column_name), value_str)
                })
                .collect::<Vec<_>>()
                .join(", ");

            let where_clause = build_where_clause_mysql(&request.primary_key_columns, row_data);

            let update_query = format!(
                "UPDATE {} SET {} WHERE {}",
                quote_identifier_mysql(&request.table_name),
                set_clause,
                where_clause
            );

            sqlx::query(&update_query).execute(&mut *tx).await?;
            edits_count += row_edits.len();
        }
    }

    // Process inserts
    for insert in &request.changes.inserts {
        let columns: Vec<String> = insert.row_data.keys()
            .map(|k| quote_identifier_mysql(k))
            .collect();

        let values: Vec<String> = insert.row_data.values()
            .map(json_value_to_sql_string_mysql)
            .collect();

        let insert_query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            quote_identifier_mysql(&request.table_name),
            columns.join(", "),
            values.join(", ")
        );

        sqlx::query(&insert_query).execute(&mut *tx).await?;
        inserts_count += 1;
    }

    tx.commit().await?;

    Ok(CommitResult {
        success: true,
        message: format!(
            "Successfully committed {} edits, {} deletes, {} inserts",
            edits_count, deletes_count, inserts_count
        ),
        edits_count,
        deletes_count,
        inserts_count,
    })
}

// Helper functions for PostgreSQL
fn quote_identifier_postgres(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace("\"", "\"\""))
}

fn build_where_clause_postgres(
    primary_keys: &[String],
    row_data: &serde_json::Map<String, serde_json::Value>,
) -> String {
    primary_keys
        .iter()
        .map(|pk| {
            let value = row_data.get(pk).unwrap_or(&serde_json::Value::Null);
            let value_str = json_value_to_sql_string_postgres(value);
            format!("{} = {}", quote_identifier_postgres(pk), value_str)
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn json_value_to_sql_string_postgres(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => b.to_string().to_uppercase(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace("'", "''")),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            format!("'{}'", serde_json::to_string(value).unwrap_or_default().replace("'", "''"))
        }
    }
}

// Helper functions for MySQL
fn quote_identifier_mysql(identifier: &str) -> String {
    format!("`{}`", identifier.replace("`", "``"))
}

fn build_where_clause_mysql(
    primary_keys: &[String],
    row_data: &serde_json::Map<String, serde_json::Value>,
) -> String {
    primary_keys
        .iter()
        .map(|pk| {
            let value = row_data.get(pk).unwrap_or(&serde_json::Value::Null);
            let value_str = json_value_to_sql_string_mysql(value);
            format!("{} = {}", quote_identifier_mysql(pk), value_str)
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn json_value_to_sql_string_mysql(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace("\\", "\\\\").replace("'", "\\'")),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            format!("'{}'", serde_json::to_string(value).unwrap_or_default().replace("\\", "\\\\").replace("'", "\\'"))
        }
    }
}
