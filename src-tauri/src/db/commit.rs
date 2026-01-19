use crate::db::connection::{ConnectionManager, DatabaseType};
use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use sqlx::QueryBuilder;

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
    let quoted_table = quote_identifier_postgres(&request.table_name);

    // Process deletes first
    for row_index in &request.changes.deletes {
        if let Some(row_data) = request.original_rows.get(*row_index) {
            let mut query_builder: QueryBuilder<sqlx::Postgres> =
                QueryBuilder::new(format!("DELETE FROM {} WHERE ", quoted_table));

            build_where_clause_with_binds_postgres(
                &mut query_builder,
                &request.primary_key_columns,
                row_data,
            );

            query_builder.build().execute(&mut *tx).await?;
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
            let mut query_builder: QueryBuilder<sqlx::Postgres> =
                QueryBuilder::new(format!("UPDATE {} SET ", quoted_table));

            // Build SET clause with bind parameters
            let mut first = true;
            for edit in &row_edits {
                if !first {
                    query_builder.push(", ");
                }
                first = false;

                query_builder.push(quote_identifier_postgres(&edit.column_name));
                query_builder.push(" = ");
                push_json_value_postgres(&mut query_builder, &edit.new_value);
            }

            query_builder.push(" WHERE ");
            build_where_clause_with_binds_postgres(
                &mut query_builder,
                &request.primary_key_columns,
                row_data,
            );

            query_builder.build().execute(&mut *tx).await?;
            edits_count += row_edits.len();
        }
    }

    // Process inserts
    for insert in &request.changes.inserts {
        if insert.row_data.is_empty() {
            continue;
        }

        let mut query_builder: QueryBuilder<sqlx::Postgres> =
            QueryBuilder::new(format!("INSERT INTO {} (", quoted_table));

        // Build column list
        let columns: Vec<String> = insert.row_data.keys()
            .map(|k| quote_identifier_postgres(k))
            .collect();
        query_builder.push(columns.join(", "));
        query_builder.push(") VALUES (");

        // Build values with bind parameters
        let mut first = true;
        for value in insert.row_data.values() {
            if !first {
                query_builder.push(", ");
            }
            first = false;
            push_json_value_postgres(&mut query_builder, value);
        }
        query_builder.push(")");

        query_builder.build().execute(&mut *tx).await?;
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
    let quoted_table = quote_identifier_mysql(&request.table_name);

    // Process deletes first
    for row_index in &request.changes.deletes {
        if let Some(row_data) = request.original_rows.get(*row_index) {
            let mut query_builder: QueryBuilder<sqlx::MySql> =
                QueryBuilder::new(format!("DELETE FROM {} WHERE ", quoted_table));

            build_where_clause_with_binds_mysql(
                &mut query_builder,
                &request.primary_key_columns,
                row_data,
            );

            query_builder.build().execute(&mut *tx).await?;
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
            let mut query_builder: QueryBuilder<sqlx::MySql> =
                QueryBuilder::new(format!("UPDATE {} SET ", quoted_table));

            // Build SET clause with bind parameters
            let mut first = true;
            for edit in &row_edits {
                if !first {
                    query_builder.push(", ");
                }
                first = false;

                query_builder.push(quote_identifier_mysql(&edit.column_name));
                query_builder.push(" = ");
                push_json_value_mysql(&mut query_builder, &edit.new_value);
            }

            query_builder.push(" WHERE ");
            build_where_clause_with_binds_mysql(
                &mut query_builder,
                &request.primary_key_columns,
                row_data,
            );

            query_builder.build().execute(&mut *tx).await?;
            edits_count += row_edits.len();
        }
    }

    // Process inserts
    for insert in &request.changes.inserts {
        if insert.row_data.is_empty() {
            continue;
        }

        let mut query_builder: QueryBuilder<sqlx::MySql> =
            QueryBuilder::new(format!("INSERT INTO {} (", quoted_table));

        // Build column list
        let columns: Vec<String> = insert.row_data.keys()
            .map(|k| quote_identifier_mysql(k))
            .collect();
        query_builder.push(columns.join(", "));
        query_builder.push(") VALUES (");

        // Build values with bind parameters
        let mut first = true;
        for value in insert.row_data.values() {
            if !first {
                query_builder.push(", ");
            }
            first = false;
            push_json_value_mysql(&mut query_builder, value);
        }
        query_builder.push(")");

        query_builder.build().execute(&mut *tx).await?;
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
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

/// Build WHERE clause with proper NULL handling using bind parameters
fn build_where_clause_with_binds_postgres(
    query_builder: &mut QueryBuilder<sqlx::Postgres>,
    primary_keys: &[String],
    row_data: &serde_json::Map<String, serde_json::Value>,
) {
    let mut first = true;
    for pk in primary_keys {
        if !first {
            query_builder.push(" AND ");
        }
        first = false;

        let value = row_data.get(pk).unwrap_or(&serde_json::Value::Null);
        query_builder.push(quote_identifier_postgres(pk));

        // Use IS NULL for null values (NULL = NULL is never true in SQL!)
        if value.is_null() {
            query_builder.push(" IS NULL");
        } else {
            query_builder.push(" = ");
            push_json_value_postgres(query_builder, value);
        }
    }
}

/// Push a JSON value as a bind parameter for PostgreSQL
fn push_json_value_postgres(query_builder: &mut QueryBuilder<sqlx::Postgres>, value: &serde_json::Value) {
    match value {
        serde_json::Value::Null => {
            query_builder.push("NULL");
        }
        serde_json::Value::Bool(b) => {
            query_builder.push_bind(*b);
        }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                query_builder.push_bind(i);
            } else if let Some(f) = n.as_f64() {
                query_builder.push_bind(f);
            } else {
                query_builder.push_bind(n.to_string());
            }
        }
        serde_json::Value::String(s) => {
            query_builder.push_bind(s.clone());
        }
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            // For JSON arrays/objects, bind as JSON string
            query_builder.push_bind(serde_json::to_string(value).unwrap_or_default());
        }
    }
}

// Helper functions for MySQL
fn quote_identifier_mysql(identifier: &str) -> String {
    format!("`{}`", identifier.replace('`', "``"))
}

/// Build WHERE clause with proper NULL handling using bind parameters
fn build_where_clause_with_binds_mysql(
    query_builder: &mut QueryBuilder<sqlx::MySql>,
    primary_keys: &[String],
    row_data: &serde_json::Map<String, serde_json::Value>,
) {
    let mut first = true;
    for pk in primary_keys {
        if !first {
            query_builder.push(" AND ");
        }
        first = false;

        let value = row_data.get(pk).unwrap_or(&serde_json::Value::Null);
        query_builder.push(quote_identifier_mysql(pk));

        // Use IS NULL for null values
        if value.is_null() {
            query_builder.push(" IS NULL");
        } else {
            query_builder.push(" = ");
            push_json_value_mysql(query_builder, value);
        }
    }
}

/// Push a JSON value as a bind parameter for MySQL
fn push_json_value_mysql(query_builder: &mut QueryBuilder<sqlx::MySql>, value: &serde_json::Value) {
    match value {
        serde_json::Value::Null => {
            query_builder.push("NULL");
        }
        serde_json::Value::Bool(b) => {
            // MySQL uses 1/0 for boolean
            query_builder.push_bind(if *b { 1i32 } else { 0i32 });
        }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                query_builder.push_bind(i);
            } else if let Some(f) = n.as_f64() {
                query_builder.push_bind(f);
            } else {
                query_builder.push_bind(n.to_string());
            }
        }
        serde_json::Value::String(s) => {
            query_builder.push_bind(s.clone());
        }
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            query_builder.push_bind(serde_json::to_string(value).unwrap_or_default());
        }
    }
}
