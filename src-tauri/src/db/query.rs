use crate::db::connection::{ConnectionManager, DatabaseType};
use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use sqlx::{Column, Row};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Map<String, serde_json::Value>>,
    pub row_count: usize,
    pub execution_time_ms: u128,
}

pub async fn execute_query(
    manager: &ConnectionManager,
    connection_id: &str,
    query: &str,
    limit: i32,
    offset: i32,
) -> AppResult<QueryResult> {
    let conn = manager.get_connection(connection_id)?;
    let start = Instant::now();

    // Add pagination to query only if not already present
    let query_upper = query.to_uppercase();
    let paginated_query = if query_upper.contains("LIMIT") {
        // Query already has LIMIT, use as-is
        query.trim_end_matches(';').to_string()
    } else {
        // Add LIMIT/OFFSET
        format!("{} LIMIT {} OFFSET {}", query.trim_end_matches(';'), limit, offset)
    };

    let result = match conn.database_type {
        DatabaseType::PostgreSQL => {
            execute_postgres_query(manager, connection_id, &paginated_query).await?
        }
        DatabaseType::MariaDB | DatabaseType::MySQL => {
            execute_mysql_query(manager, connection_id, &paginated_query).await?
        }
    };

    let execution_time_ms = start.elapsed().as_millis();

    Ok(QueryResult {
        columns: result.0,
        rows: result.1,
        row_count: result.2,
        execution_time_ms,
    })
}

async fn execute_postgres_query(
    manager: &ConnectionManager,
    connection_id: &str,
    query: &str,
) -> AppResult<(Vec<String>, Vec<serde_json::Map<String, serde_json::Value>>, usize)> {
    let pool = manager.get_pool_postgres(connection_id).await?;

    let rows = sqlx::query(query).fetch_all(&pool).await?;

    if rows.is_empty() {
        return Ok((vec![], vec![], 0));
    }

    // Get column names
    let columns: Vec<String> = rows[0]
        .columns()
        .iter()
        .map(|col| col.name().to_string())
        .collect();

    // Convert rows to JSON
    let mut result_rows = Vec::new();

    for row in &rows {
        let mut row_map = serde_json::Map::new();

        for (idx, column) in row.columns().iter().enumerate() {
            let col_name = column.name().to_string();

            // Try to get the value - handle different types
            let value = if let Ok(val) = row.try_get::<Option<String>, _>(idx) {
                serde_json::Value::String(val.unwrap_or_default())
            } else if let Ok(val) = row.try_get::<Option<i32>, _>(idx) {
                val.map(|v| serde_json::Value::Number(v.into()))
                    .unwrap_or(serde_json::Value::Null)
            } else if let Ok(val) = row.try_get::<Option<i64>, _>(idx) {
                val.map(|v| serde_json::Value::Number(v.into()))
                    .unwrap_or(serde_json::Value::Null)
            } else if let Ok(val) = row.try_get::<Option<f64>, _>(idx) {
                val.and_then(|v| serde_json::Number::from_f64(v))
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else if let Ok(val) = row.try_get::<Option<bool>, _>(idx) {
                val.map(serde_json::Value::Bool)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                // Default to empty string for unsupported types
                serde_json::Value::String(String::from("<unsupported type>"))
            };

            row_map.insert(col_name, value);
        }

        result_rows.push(row_map);
    }

    Ok((columns, result_rows, rows.len()))
}

async fn execute_mysql_query(
    manager: &ConnectionManager,
    connection_id: &str,
    query: &str,
) -> AppResult<(Vec<String>, Vec<serde_json::Map<String, serde_json::Value>>, usize)> {
    let pool = manager.get_pool_mysql(connection_id).await?;

    let rows = sqlx::query(query).fetch_all(&pool).await?;

    if rows.is_empty() {
        return Ok((vec![], vec![], 0));
    }

    // Get column names
    let columns: Vec<String> = rows[0]
        .columns()
        .iter()
        .map(|col| col.name().to_string())
        .collect();

    // Convert rows to JSON
    let mut result_rows = Vec::new();

    for row in &rows {
        let mut row_map = serde_json::Map::new();

        for (idx, column) in row.columns().iter().enumerate() {
            let col_name = column.name().to_string();

            // Try to get the value - handle different types
            let value = if let Ok(val) = row.try_get::<Option<String>, _>(idx) {
                serde_json::Value::String(val.unwrap_or_default())
            } else if let Ok(val) = row.try_get::<Option<i32>, _>(idx) {
                val.map(|v| serde_json::Value::Number(v.into()))
                    .unwrap_or(serde_json::Value::Null)
            } else if let Ok(val) = row.try_get::<Option<i64>, _>(idx) {
                val.map(|v| serde_json::Value::Number(v.into()))
                    .unwrap_or(serde_json::Value::Null)
            } else if let Ok(val) = row.try_get::<Option<f64>, _>(idx) {
                val.and_then(|v| serde_json::Number::from_f64(v))
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else if let Ok(val) = row.try_get::<Option<bool>, _>(idx) {
                val.map(serde_json::Value::Bool)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                // Default to empty string for unsupported types
                serde_json::Value::String(String::from("<unsupported type>"))
            };

            row_map.insert(col_name, value);
        }

        result_rows.push(row_map);
    }

    Ok((columns, result_rows, rows.len()))
}
