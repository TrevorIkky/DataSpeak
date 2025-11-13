use crate::db::connection::{ConnectionManager, DatabaseType};
use crate::error::AppResult;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};
use sqlx::{Column, Row, TypeInfo, ValueRef};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyMetadata {
    pub referenced_table: String,
    pub referenced_column: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMetadata {
    pub name: String,
    pub data_type: String,
    pub enum_values: Option<Vec<String>>,
    pub foreign_key: Option<ForeignKeyMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub column_metadata: Vec<ColumnMetadata>,
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
        column_metadata: result.1,
        rows: result.2,
        row_count: result.3,
        execution_time_ms,
    })
}

pub async fn execute_table_query(
    manager: &ConnectionManager,
    connection_id: &str,
    table_name: &str,
    filter_column: Option<String>,
    filter_value: Option<serde_json::Value>,
    limit: i32,
    offset: i32,
) -> AppResult<QueryResult> {
    let conn = manager.get_connection(connection_id)?;
    let start = Instant::now();

    // Build the base query
    let mut query = format!("SELECT * FROM {}", table_name);

    // Add WHERE clause if filter is provided
    if let (Some(column), Some(value)) = (filter_column, filter_value) {
        let where_clause = match value {
            serde_json::Value::Null => format!("{} IS NULL", column),
            serde_json::Value::Bool(b) => format!("{} = {}", column, b),
            serde_json::Value::Number(n) => format!("{} = {}", column, n),
            serde_json::Value::String(s) => {
                // Escape single quotes by doubling them (SQL standard)
                let escaped = s.replace("'", "''");
                format!("{} = '{}'", column, escaped)
            }
            _ => {
                // For arrays and objects, convert to string and escape
                let s = value.to_string();
                let escaped = s.replace("'", "''");
                format!("{} = '{}'", column, escaped)
            }
        };
        query.push_str(&format!(" WHERE {}", where_clause));
    }

    // Add pagination
    query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

    // Reuse existing query execution logic
    let result = match conn.database_type {
        DatabaseType::PostgreSQL => {
            execute_postgres_query(manager, connection_id, &query).await?
        }
        DatabaseType::MariaDB | DatabaseType::MySQL => {
            execute_mysql_query(manager, connection_id, &query).await?
        }
    };

    let execution_time_ms = start.elapsed().as_millis();

    Ok(QueryResult {
        columns: result.0,
        column_metadata: result.1,
        rows: result.2,
        row_count: result.3,
        execution_time_ms,
    })
}

async fn execute_postgres_query(
    manager: &ConnectionManager,
    connection_id: &str,
    query: &str,
) -> AppResult<(Vec<String>, Vec<ColumnMetadata>, Vec<serde_json::Map<String, serde_json::Value>>, usize)> {
    let pool = manager.get_pool_postgres(connection_id).await?;

    let rows = sqlx::query(query).fetch_all(&pool).await?;

    // Try to extract table name and get FK metadata
    let fk_map = if let Some(table_name) = extract_table_name(query) {
        // Default to 'public' schema
        get_postgres_fk_metadata(&pool, &table_name, "public")
            .await
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    // Get column names and metadata from first row, or try to get column info even with no rows
    let (columns, column_metadata): (Vec<String>, Vec<ColumnMetadata>) = if !rows.is_empty() {
        let cols: Vec<_> = rows[0].columns().iter().map(|col| {
            let name = col.name().to_string();
            let data_type = col.type_info().name().to_string();
            let foreign_key = fk_map.get(&name).cloned();
            (name.clone(), ColumnMetadata {
                name,
                data_type,
                enum_values: None, // PostgreSQL enums would need schema query
                foreign_key,
            })
        }).collect();
        (cols.iter().map(|(name, _)| name.clone()).collect(),
         cols.into_iter().map(|(_, meta)| meta).collect())
    } else {
        // No rows, try to prepare the query to get column metadata
        match sqlx::query(query).fetch_optional(&pool).await {
            Ok(Some(row)) => {
                let cols: Vec<_> = row.columns().iter().map(|col| {
                    let name = col.name().to_string();
                    let data_type = col.type_info().name().to_string();
                    let foreign_key = fk_map.get(&name).cloned();
                    (name.clone(), ColumnMetadata {
                        name,
                        data_type,
                        enum_values: None,
                        foreign_key,
                    })
                }).collect();
                (cols.iter().map(|(name, _)| name.clone()).collect(),
                 cols.into_iter().map(|(_, meta)| meta).collect())
            }
            _ => {
                // Can't get column info
                (vec![], vec![])
            }
        }
    };

    if rows.is_empty() {
        return Ok((columns, column_metadata, vec![], 0));
    }

    // Convert rows to JSON
    let mut result_rows = Vec::new();

    for row in &rows {
        let mut row_map = serde_json::Map::new();

        for (idx, column) in row.columns().iter().enumerate() {
            let col_name = column.name().to_string();
            let col_type = column.type_info().name();

            // Check if the value is NULL first
            let raw_value = row.try_get_raw(idx)?;
            if raw_value.is_null() {
                row_map.insert(col_name, serde_json::Value::Null);
                continue;
            }

            // Try to get the value based on PostgreSQL type
            let value = match col_type {
                // Boolean
                "BOOL" => row.try_get::<bool, _>(idx)
                    .map(serde_json::Value::Bool)
                    .unwrap_or(serde_json::Value::Null),

                // Integer types
                "INT2" | "SMALLINT" | "SMALLSERIAL" => row.try_get::<i16, _>(idx)
                    .map(|v| serde_json::Value::Number(v.into()))
                    .unwrap_or(serde_json::Value::Null),

                "INT4" | "INT" | "SERIAL" => row.try_get::<i32, _>(idx)
                    .map(|v| serde_json::Value::Number(v.into()))
                    .unwrap_or(serde_json::Value::Null),

                "INT8" | "BIGINT" | "BIGSERIAL" => row.try_get::<i64, _>(idx)
                    .map(|v| serde_json::Value::Number(v.into()))
                    .unwrap_or(serde_json::Value::Null),

                // Float types
                "FLOAT4" | "REAL" => row.try_get::<f32, _>(idx)
                    .ok()
                    .and_then(|v| serde_json::Number::from_f64(v as f64))
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null),

                "FLOAT8" | "DOUBLE PRECISION" => row.try_get::<f64, _>(idx)
                    .ok()
                    .and_then(|v| serde_json::Number::from_f64(v))
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null),

                // Numeric/Decimal - convert to string to preserve precision
                "NUMERIC" | "DECIMAL" => {
                    // Try as string first to preserve precision
                    if let Ok(val) = row.try_get::<String, _>(idx) {
                        serde_json::Value::String(val)
                    } else {
                        serde_json::Value::Null
                    }
                }

                // Date and Time types
                "DATE" => row.try_get::<NaiveDate, _>(idx)
                    .map(|v| serde_json::Value::String(v.to_string()))
                    .unwrap_or(serde_json::Value::Null),

                "TIME" => row.try_get::<NaiveTime, _>(idx)
                    .map(|v| serde_json::Value::String(v.to_string()))
                    .unwrap_or(serde_json::Value::Null),

                "TIMESTAMP" => row.try_get::<NaiveDateTime, _>(idx)
                    .map(|v| serde_json::Value::String(v.to_string()))
                    .unwrap_or(serde_json::Value::Null),

                "TIMESTAMPTZ" | "TIMESTAMP WITH TIME ZONE" => {
                    row.try_get::<DateTime<chrono::Utc>, _>(idx)
                        .map(|v| serde_json::Value::String(v.to_rfc3339()))
                        .unwrap_or(serde_json::Value::Null)
                }

                // UUID
                "UUID" => row.try_get::<uuid::Uuid, _>(idx)
                    .map(|v| serde_json::Value::String(v.to_string()))
                    .unwrap_or(serde_json::Value::Null),

                // JSON types
                "JSON" | "JSONB" => row.try_get::<serde_json::Value, _>(idx)
                    .unwrap_or(serde_json::Value::Null),

                // Array types - convert to JSON array
                "_INT4" | "_INT8" | "_TEXT" | "_VARCHAR" | "_BOOL" | "_FLOAT4" | "_FLOAT8" => {
                    // Arrays are complex, try to get as JSON string
                    if let Ok(val) = row.try_get::<String, _>(idx) {
                        serde_json::Value::String(val)
                    } else {
                        serde_json::Value::Null
                    }
                }

                // Binary data - convert to hex string
                "BYTEA" => row.try_get::<Vec<u8>, _>(idx)
                    .map(|bytes| serde_json::Value::String(
                        format!("0x{}", hex::encode(bytes))
                    ))
                    .unwrap_or(serde_json::Value::Null),

                // PostGIS Geometry types - try to get as string (WKT format)
                // Note: Use ST_AsText(geom_column) in queries to get WKT
                "GEOMETRY" | "GEOGRAPHY" | "POINT" | "LINESTRING" | "POLYGON" |
                "MULTIPOINT" | "MULTILINESTRING" | "MULTIPOLYGON" | "GEOMETRYCOLLECTION" => {
                    // PostGIS types need ST_AsText() to convert to WKT
                    // If already converted, we'll get string
                    // Otherwise, we get binary which we can't easily parse
                    if let Ok(wkt) = row.try_get::<String, _>(idx) {
                        serde_json::Value::String(wkt)
                    } else if let Ok(bytes) = row.try_get::<Vec<u8>, _>(idx) {
                        // PostGIS stores as EWKB (Extended Well-Known Binary)
                        // Indicate geometry data is present but needs ST_AsText()
                        serde_json::Value::String(format!("<PostGIS geometry: {} bytes - use ST_AsText() to view>", bytes.len()))
                    } else {
                        serde_json::Value::Null
                    }
                }

                // Text types (including VARCHAR, CHAR, TEXT, etc.)
                _ => {
                    // Default: try string, then numeric types, then give up
                    if let Ok(val) = row.try_get::<String, _>(idx) {
                        serde_json::Value::String(val)
                    } else if let Ok(val) = row.try_get::<i64, _>(idx) {
                        serde_json::Value::Number(val.into())
                    } else if let Ok(val) = row.try_get::<f64, _>(idx) {
                        serde_json::Number::from_f64(val)
                            .map(serde_json::Value::Number)
                            .unwrap_or(serde_json::Value::Null)
                    } else if let Ok(val) = row.try_get::<bool, _>(idx) {
                        serde_json::Value::Bool(val)
                    } else {
                        serde_json::Value::String(format!("<unsupported: {}>", col_type))
                    }
                }
            };

            row_map.insert(col_name, value);
        }

        result_rows.push(row_map);
    }

    Ok((columns, column_metadata, result_rows, rows.len()))
}

// Helper function to get foreign key metadata for PostgreSQL
async fn get_postgres_fk_metadata(
    pool: &sqlx::PgPool,
    table_name: &str,
    schema_name: &str,
) -> AppResult<HashMap<String, ForeignKeyMetadata>> {
    let fk_query = r#"
        SELECT
            kcu.column_name,
            ccu.table_name AS referenced_table,
            ccu.column_name AS referenced_column
        FROM information_schema.table_constraints AS tc
        JOIN information_schema.key_column_usage AS kcu
          ON tc.constraint_name = kcu.constraint_name
          AND tc.table_schema = kcu.table_schema
        JOIN information_schema.constraint_column_usage AS ccu
          ON ccu.constraint_name = tc.constraint_name
          AND ccu.table_schema = tc.table_schema
        WHERE tc.constraint_type = 'FOREIGN KEY'
          AND tc.table_name = $1
          AND tc.table_schema = $2
    "#;

    let rows = sqlx::query(fk_query)
        .bind(table_name)
        .bind(schema_name)
        .fetch_all(pool)
        .await?;

    let mut fk_map = HashMap::new();
    for row in rows {
        let column_name: String = row.try_get("column_name")?;
        let referenced_table: String = row.try_get("referenced_table")?;
        let referenced_column: String = row.try_get("referenced_column")?;

        fk_map.insert(
            column_name,
            ForeignKeyMetadata {
                referenced_table,
                referenced_column,
            },
        );
    }

    Ok(fk_map)
}

// Helper function to get foreign key metadata for MySQL
async fn get_mysql_fk_metadata(
    pool: &sqlx::MySqlPool,
    table_name: &str,
    database_name: &str,
) -> AppResult<HashMap<String, ForeignKeyMetadata>> {
    let fk_query = r#"
        SELECT
            COLUMN_NAME as column_name,
            REFERENCED_TABLE_NAME as referenced_table,
            REFERENCED_COLUMN_NAME as referenced_column
        FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE
        WHERE TABLE_SCHEMA = ?
          AND TABLE_NAME = ?
          AND REFERENCED_TABLE_NAME IS NOT NULL
    "#;

    let rows = sqlx::query(fk_query)
        .bind(database_name)
        .bind(table_name)
        .fetch_all(pool)
        .await?;

    let mut fk_map = HashMap::new();
    for row in rows {
        let column_name: String = row.try_get("column_name")?;
        let referenced_table: String = row.try_get("referenced_table")?;
        let referenced_column: String = row.try_get("referenced_column")?;

        fk_map.insert(
            column_name,
            ForeignKeyMetadata {
                referenced_table,
                referenced_column,
            },
        );
    }

    Ok(fk_map)
}

// Helper to extract table name from simple SELECT queries
fn extract_table_name(query: &str) -> Option<String> {
    let query_upper = query.to_uppercase();

    // Simple pattern: SELECT ... FROM table_name
    if let Some(from_idx) = query_upper.find("FROM") {
        let after_from = &query[from_idx + 4..].trim();
        // Get the first word after FROM (table name)
        let table_name = after_from
            .split_whitespace()
            .next()?
            .trim_matches(|c| c == '`' || c == '"' || c == '\'' || c == ';')
            .to_string();

        // Don't include schema prefix, just table name
        if let Some(dot_idx) = table_name.rfind('.') {
            return Some(table_name[dot_idx + 1..].to_string());
        }

        return Some(table_name);
    }

    None
}

async fn execute_mysql_query(
    manager: &ConnectionManager,
    connection_id: &str,
    query: &str,
) -> AppResult<(Vec<String>, Vec<ColumnMetadata>, Vec<serde_json::Map<String, serde_json::Value>>, usize)> {
    let pool = manager.get_pool_mysql(connection_id).await?;

    let rows = sqlx::query(query).fetch_all(&pool).await?;

    // Get current database name for FK queries
    let database_name: (String,) = sqlx::query_as("SELECT DATABASE()")
        .fetch_one(&pool)
        .await?;
    let database_name = database_name.0;

    // Try to extract table name and get FK metadata
    let fk_map = if let Some(table_name) = extract_table_name(query) {
        get_mysql_fk_metadata(&pool, &table_name, &database_name)
            .await
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    // Get column names and metadata from first row, or try to get column info even with no rows
    let (columns, column_metadata): (Vec<String>, Vec<ColumnMetadata>) = if !rows.is_empty() {
        let cols: Vec<_> = rows[0].columns().iter().map(|col| {
            let name = col.name().to_string();
            let data_type = col.type_info().name().to_string();
            let foreign_key = fk_map.get(&name).cloned();
            (name.clone(), ColumnMetadata {
                name,
                data_type,
                enum_values: None, // MySQL enums would need SHOW COLUMNS query
                foreign_key,
            })
        }).collect();
        (cols.iter().map(|(name, _)| name.clone()).collect(),
         cols.into_iter().map(|(_, meta)| meta).collect())
    } else {
        // No rows, try to prepare the query to get column metadata
        match sqlx::query(query).fetch_optional(&pool).await {
            Ok(Some(row)) => {
                let cols: Vec<_> = row.columns().iter().map(|col| {
                    let name = col.name().to_string();
                    let data_type = col.type_info().name().to_string();
                    let foreign_key = fk_map.get(&name).cloned();
                    (name.clone(), ColumnMetadata {
                        name,
                        data_type,
                        enum_values: None,
                        foreign_key,
                    })
                }).collect();
                (cols.iter().map(|(name, _)| name.clone()).collect(),
                 cols.into_iter().map(|(_, meta)| meta).collect())
            }
            _ => {
                // Can't get column info
                (vec![], vec![])
            }
        }
    };

    if rows.is_empty() {
        return Ok((columns, column_metadata, vec![], 0));
    }

    // Convert rows to JSON
    let mut result_rows = Vec::new();

    for row in &rows {
        let mut row_map = serde_json::Map::new();

        for (idx, column) in row.columns().iter().enumerate() {
            let col_name = column.name().to_string();
            let col_type = column.type_info().name();

            // Check if the value is NULL first
            let raw_value = row.try_get_raw(idx)?;
            if raw_value.is_null() {
                row_map.insert(col_name, serde_json::Value::Null);
                continue;
            }

            // Try to get the value based on MySQL type
            let value = match col_type {
                // Boolean/Tiny Int
                "BOOLEAN" | "TINYINT(1)" => row.try_get::<bool, _>(idx)
                    .map(serde_json::Value::Bool)
                    .or_else(|_| row.try_get::<i8, _>(idx).map(|v| serde_json::Value::Number(v.into())))
                    .unwrap_or(serde_json::Value::Null),

                // Integer types
                "TINYINT" => row.try_get::<i8, _>(idx)
                    .map(|v| serde_json::Value::Number(v.into()))
                    .unwrap_or(serde_json::Value::Null),

                "SMALLINT" => row.try_get::<i16, _>(idx)
                    .map(|v| serde_json::Value::Number(v.into()))
                    .unwrap_or(serde_json::Value::Null),

                "MEDIUMINT" | "INT" | "INTEGER" => row.try_get::<i32, _>(idx)
                    .map(|v| serde_json::Value::Number(v.into()))
                    .unwrap_or(serde_json::Value::Null),

                "BIGINT" => row.try_get::<i64, _>(idx)
                    .map(|v| serde_json::Value::Number(v.into()))
                    .unwrap_or(serde_json::Value::Null),

                // Unsigned Integer types
                "TINYINT UNSIGNED" => row.try_get::<u8, _>(idx)
                    .map(|v| serde_json::Value::Number(v.into()))
                    .unwrap_or(serde_json::Value::Null),

                "SMALLINT UNSIGNED" => row.try_get::<u16, _>(idx)
                    .map(|v| serde_json::Value::Number(v.into()))
                    .unwrap_or(serde_json::Value::Null),

                "MEDIUMINT UNSIGNED" | "INT UNSIGNED" => row.try_get::<u32, _>(idx)
                    .map(|v| serde_json::Value::Number(v.into()))
                    .unwrap_or(serde_json::Value::Null),

                "BIGINT UNSIGNED" => row.try_get::<u64, _>(idx)
                    .map(|v| serde_json::Value::Number(v.into()))
                    .unwrap_or(serde_json::Value::Null),

                // Float types
                "FLOAT" => row.try_get::<f32, _>(idx)
                    .ok()
                    .and_then(|v| serde_json::Number::from_f64(v as f64))
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null),

                "DOUBLE" | "REAL" => row.try_get::<f64, _>(idx)
                    .ok()
                    .and_then(|v| serde_json::Number::from_f64(v))
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null),

                // Decimal/Numeric - convert to string to preserve precision
                "DECIMAL" | "NUMERIC" => {
                    if let Ok(val) = row.try_get::<String, _>(idx) {
                        serde_json::Value::String(val)
                    } else {
                        serde_json::Value::Null
                    }
                }

                // Date and Time types
                "DATE" => row.try_get::<NaiveDate, _>(idx)
                    .map(|v| serde_json::Value::String(v.to_string()))
                    .unwrap_or(serde_json::Value::Null),

                "TIME" => row.try_get::<NaiveTime, _>(idx)
                    .map(|v| serde_json::Value::String(v.to_string()))
                    .or_else(|_| {
                        // MySQL TIME can be negative or > 24h, fallback to string
                        row.try_get::<String, _>(idx).map(serde_json::Value::String)
                    })
                    .unwrap_or(serde_json::Value::Null),

                "DATETIME" | "TIMESTAMP" => row.try_get::<NaiveDateTime, _>(idx)
                    .map(|v| serde_json::Value::String(v.to_string()))
                    .or_else(|_| {
                        // Fallback to string for edge cases
                        row.try_get::<String, _>(idx).map(serde_json::Value::String)
                    })
                    .unwrap_or(serde_json::Value::Null),

                "YEAR" => row.try_get::<i16, _>(idx)
                    .map(|v| serde_json::Value::Number(v.into()))
                    .or_else(|_| row.try_get::<String, _>(idx).map(serde_json::Value::String))
                    .unwrap_or(serde_json::Value::Null),

                // JSON type
                "JSON" => row.try_get::<serde_json::Value, _>(idx)
                    .or_else(|_| row.try_get::<String, _>(idx).and_then(|s| {
                        serde_json::from_str(&s).map_err(|_| sqlx::Error::ColumnNotFound("json".to_string()))
                    }))
                    .unwrap_or(serde_json::Value::Null),

                // Binary types - convert to hex string
                "BINARY" | "VARBINARY" | "TINYBLOB" | "BLOB" | "MEDIUMBLOB" | "LONGBLOB" => {
                    row.try_get::<Vec<u8>, _>(idx)
                        .map(|bytes| {
                            // Limit display for large binary data
                            if bytes.len() > 256 {
                                serde_json::Value::String(format!("0x{}... ({} bytes)", hex::encode(&bytes[..256]), bytes.len()))
                            } else {
                                serde_json::Value::String(format!("0x{}", hex::encode(bytes)))
                            }
                        })
                        .unwrap_or(serde_json::Value::Null)
                }

                // ENUM and SET - return as string
                "ENUM" | "SET" => row.try_get::<String, _>(idx)
                    .map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null),

                // Spatial/Geometry types (MySQL) - convert to WKT string representation
                "GEOMETRY" | "POINT" | "LINESTRING" | "POLYGON" | "MULTIPOINT" |
                "MULTILINESTRING" | "MULTIPOLYGON" | "GEOMETRYCOLLECTION" => {
                    // Try to get as WKT string first
                    if let Ok(wkt) = row.try_get::<String, _>(idx) {
                        serde_json::Value::String(wkt)
                    } else if let Ok(bytes) = row.try_get::<Vec<u8>, _>(idx) {
                        // MySQL stores geometry as WKB (Well-Known Binary)
                        // For now, just indicate we have geometry data
                        serde_json::Value::String(format!("<geometry: {} bytes>", bytes.len()))
                    } else {
                        serde_json::Value::Null
                    }
                }

                // Text types (VARCHAR, CHAR, TEXT, etc.) and default
                _ => {
                    // Default: try string, then numeric types, then give up
                    if let Ok(val) = row.try_get::<String, _>(idx) {
                        serde_json::Value::String(val)
                    } else if let Ok(val) = row.try_get::<i64, _>(idx) {
                        serde_json::Value::Number(val.into())
                    } else if let Ok(val) = row.try_get::<f64, _>(idx) {
                        serde_json::Number::from_f64(val)
                            .map(serde_json::Value::Number)
                            .unwrap_or(serde_json::Value::Null)
                    } else if let Ok(val) = row.try_get::<bool, _>(idx) {
                        serde_json::Value::Bool(val)
                    } else {
                        serde_json::Value::String(format!("<unsupported: {}>", col_type))
                    }
                }
            };

            row_map.insert(col_name, value);
        }

        result_rows.push(row_map);
    }

    Ok((columns, column_metadata, result_rows, rows.len()))
}
