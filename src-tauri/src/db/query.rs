use crate::db::connection::{ConnectionManager, DatabaseType};
use crate::error::AppResult;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};
use sqlx::{Column, Row, TypeInfo, ValueRef};
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

    // Get column names from first row, or try to get column info even with no rows
    let columns: Vec<String> = if !rows.is_empty() {
        rows[0]
            .columns()
            .iter()
            .map(|col| col.name().to_string())
            .collect()
    } else {
        // No rows, try to prepare the query to get column metadata
        // Use fetch_optional which will give us row structure even if empty
        match sqlx::query(query).fetch_optional(&pool).await {
            Ok(Some(row)) => {
                row.columns()
                    .iter()
                    .map(|col| col.name().to_string())
                    .collect()
            }
            _ => {
                // Can't get column info
                vec![]
            }
        }
    };

    if rows.is_empty() {
        return Ok((columns, vec![], 0));
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

    Ok((columns, result_rows, rows.len()))
}

async fn execute_mysql_query(
    manager: &ConnectionManager,
    connection_id: &str,
    query: &str,
) -> AppResult<(Vec<String>, Vec<serde_json::Map<String, serde_json::Value>>, usize)> {
    let pool = manager.get_pool_mysql(connection_id).await?;

    let rows = sqlx::query(query).fetch_all(&pool).await?;

    // Get column names from first row, or try to get column info even with no rows
    let columns: Vec<String> = if !rows.is_empty() {
        rows[0]
            .columns()
            .iter()
            .map(|col| col.name().to_string())
            .collect()
    } else {
        // No rows, try to prepare the query to get column metadata
        // Use fetch_optional which will give us row structure even if empty
        match sqlx::query(query).fetch_optional(&pool).await {
            Ok(Some(row)) => {
                row.columns()
                    .iter()
                    .map(|col| col.name().to_string())
                    .collect()
            }
            _ => {
                // Can't get column info
                vec![]
            }
        }
    };

    if rows.is_empty() {
        return Ok((columns, vec![], 0));
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

    Ok((columns, result_rows, rows.len()))
}
