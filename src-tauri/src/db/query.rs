use crate::db::connection::{ConnectionManager, DatabaseType};
use crate::error::AppResult;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};
use sqlx::postgres::types::{Oid, PgInterval};
use sqlx::types::ipnetwork::IpNetwork;
use sqlx::{Column, Row, TypeInfo, ValueRef};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Instant;

/// Quote an identifier for PostgreSQL (uses double quotes)
fn quote_identifier_postgres(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

/// Quote an identifier for MySQL (uses backticks)
fn quote_identifier_mysql(identifier: &str) -> String {
    format!("`{}`", identifier.replace('`', "``"))
}

/// Quote an identifier based on database type
fn quote_identifier(identifier: &str, db_type: &DatabaseType) -> String {
    match db_type {
        DatabaseType::PostgreSQL => quote_identifier_postgres(identifier),
        DatabaseType::MariaDB | DatabaseType::MySQL => quote_identifier_mysql(identifier),
    }
}

/// Convert float to JSON, handling special values (NaN, Infinity)
/// serde_json::Number::from_f64() returns None for NaN/Infinity, so we
/// represent them as special string values for data integrity
fn float_to_json(val: f64) -> serde_json::Value {
    if val.is_nan() {
        serde_json::Value::String("NaN".to_string())
    } else if val.is_infinite() {
        if val.is_sign_positive() {
            serde_json::Value::String("Infinity".to_string())
        } else {
            serde_json::Value::String("-Infinity".to_string())
        }
    } else {
        serde_json::Number::from_f64(val)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null)
    }
}

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

/// Consolidated table metadata for efficient lookup during row processing
#[derive(Default)]
struct TableMetadata {
    foreign_keys: HashMap<String, ForeignKeyMetadata>,
    enum_values: HashMap<String, Vec<String>>,
}

impl TableMetadata {
    fn get_column_metadata(&self, name: &str, data_type: String) -> ColumnMetadata {
        ColumnMetadata {
            name: name.to_string(),
            data_type,
            enum_values: self.enum_values.get(name).cloned(),
            foreign_key: self.foreign_keys.get(name).cloned(),
        }
    }
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

    // Quote table name to prevent SQL injection
    let quoted_table = quote_identifier(table_name, &conn.database_type);

    // Execute with parameterized filter if provided
    let result = match &conn.database_type {
        DatabaseType::PostgreSQL => {
            execute_postgres_table_query(
                manager, connection_id, &quoted_table, table_name,
                filter_column, filter_value, limit, offset
            ).await?
        }
        DatabaseType::MariaDB | DatabaseType::MySQL => {
            execute_mysql_table_query(
                manager, connection_id, &quoted_table, table_name,
                filter_column, filter_value, limit, offset
            ).await?
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

/// Execute a PostgreSQL table query with parameterized filter
async fn execute_postgres_table_query(
    manager: &ConnectionManager,
    connection_id: &str,
    quoted_table: &str,
    raw_table_name: &str,
    filter_column: Option<String>,
    filter_value: Option<serde_json::Value>,
    limit: i32,
    offset: i32,
) -> AppResult<(Vec<String>, Vec<ColumnMetadata>, Vec<serde_json::Map<String, serde_json::Value>>, usize)> {
    let pool = manager.get_pool_postgres(connection_id).await?;

    // Build query with parameterized filter
    let (query, bind_value) = if let (Some(column), Some(value)) = (filter_column, filter_value) {
        let quoted_column = quote_identifier_postgres(&column);
        if value.is_null() {
            // NULL requires IS NULL, not = $1
            let q = format!(
                "SELECT * FROM {} WHERE {} IS NULL LIMIT {} OFFSET {}",
                quoted_table, quoted_column, limit, offset
            );
            (q, None)
        } else {
            let q = format!(
                "SELECT * FROM {} WHERE {} = $1 LIMIT {} OFFSET {}",
                quoted_table, quoted_column, limit, offset
            );
            (q, Some(value))
        }
    } else {
        let q = format!("SELECT * FROM {} LIMIT {} OFFSET {}", quoted_table, limit, offset);
        (q, None)
    };

    // Execute with or without bind parameter
    let rows = if let Some(val) = bind_value {
        // Convert JSON value to appropriate bind type
        match val {
            serde_json::Value::Bool(b) => {
                sqlx::query(&query).bind(b).fetch_all(&pool).await?
            }
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    sqlx::query(&query).bind(i).fetch_all(&pool).await?
                } else if let Some(f) = n.as_f64() {
                    sqlx::query(&query).bind(f).fetch_all(&pool).await?
                } else {
                    sqlx::query(&query).bind(n.to_string()).fetch_all(&pool).await?
                }
            }
            serde_json::Value::String(s) => {
                sqlx::query(&query).bind(s).fetch_all(&pool).await?
            }
            _ => {
                // For arrays/objects, bind as JSON string
                sqlx::query(&query).bind(val.to_string()).fetch_all(&pool).await?
            }
        }
    } else {
        sqlx::query(&query).fetch_all(&pool).await?
    };

    // Fetch FK and enum metadata in parallel
    let (fk_result, enum_result) = tokio::join!(
        get_postgres_fk_metadata(&pool, raw_table_name, "public"),
        get_postgres_enum_values(&pool, raw_table_name, "public")
    );

    let metadata = TableMetadata {
        foreign_keys: fk_result.unwrap_or_default(),
        enum_values: enum_result.unwrap_or_default(),
    };

    process_postgres_rows(rows, metadata).await
}

/// Execute a MySQL table query with parameterized filter
async fn execute_mysql_table_query(
    manager: &ConnectionManager,
    connection_id: &str,
    quoted_table: &str,
    raw_table_name: &str,
    filter_column: Option<String>,
    filter_value: Option<serde_json::Value>,
    limit: i32,
    offset: i32,
) -> AppResult<(Vec<String>, Vec<ColumnMetadata>, Vec<serde_json::Map<String, serde_json::Value>>, usize)> {
    let pool = manager.get_pool_mysql(connection_id).await?;

    // Build query with parameterized filter
    let (query, bind_value) = if let (Some(column), Some(value)) = (filter_column, filter_value) {
        let quoted_column = quote_identifier_mysql(&column);
        if value.is_null() {
            let q = format!(
                "SELECT * FROM {} WHERE {} IS NULL LIMIT {} OFFSET {}",
                quoted_table, quoted_column, limit, offset
            );
            (q, None)
        } else {
            let q = format!(
                "SELECT * FROM {} WHERE {} = ? LIMIT {} OFFSET {}",
                quoted_table, quoted_column, limit, offset
            );
            (q, Some(value))
        }
    } else {
        let q = format!("SELECT * FROM {} LIMIT {} OFFSET {}", quoted_table, limit, offset);
        (q, None)
    };

    // Execute with or without bind parameter
    let rows = if let Some(val) = bind_value {
        match val {
            serde_json::Value::Bool(b) => {
                sqlx::query(&query).bind(b).fetch_all(&pool).await?
            }
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    sqlx::query(&query).bind(i).fetch_all(&pool).await?
                } else if let Some(f) = n.as_f64() {
                    sqlx::query(&query).bind(f).fetch_all(&pool).await?
                } else {
                    sqlx::query(&query).bind(n.to_string()).fetch_all(&pool).await?
                }
            }
            serde_json::Value::String(s) => {
                sqlx::query(&query).bind(s).fetch_all(&pool).await?
            }
            _ => {
                sqlx::query(&query).bind(val.to_string()).fetch_all(&pool).await?
            }
        }
    } else {
        sqlx::query(&query).fetch_all(&pool).await?
    };

    // Get database name and fetch FK/enum metadata in parallel
    let database_name: (String,) = sqlx::query_as("SELECT DATABASE()")
        .fetch_one(&pool)
        .await?;
    let db_name = &database_name.0;

    let (fk_result, enum_result) = tokio::join!(
        get_mysql_fk_metadata(&pool, raw_table_name, db_name),
        get_mysql_enum_values(&pool, raw_table_name, db_name)
    );

    let metadata = TableMetadata {
        foreign_keys: fk_result.unwrap_or_default(),
        enum_values: enum_result.unwrap_or_default(),
    };

    process_mysql_rows(rows, metadata).await
}

/// Process PostgreSQL rows into JSON format with metadata
async fn process_postgres_rows(
    rows: Vec<sqlx::postgres::PgRow>,
    metadata: TableMetadata,
) -> AppResult<(Vec<String>, Vec<ColumnMetadata>, Vec<serde_json::Map<String, serde_json::Value>>, usize)> {
    if rows.is_empty() {
        return Ok((vec![], vec![], vec![], 0));
    }

    // Build column metadata from first row
    let (columns, column_metadata): (Vec<String>, Vec<ColumnMetadata>) = rows[0]
        .columns()
        .iter()
        .map(|col| {
            let name = col.name().to_string();
            let data_type = col.type_info().name().to_string();
            (name.clone(), metadata.get_column_metadata(&name, data_type))
        })
        .unzip();

    // Convert rows to JSON with pre-allocated capacity
    let row_count = rows.len();
    let col_count = columns.len();
    let mut result_rows = Vec::with_capacity(row_count);

    for row in &rows {
        let mut row_map = serde_json::Map::with_capacity(col_count);
        for (idx, column) in row.columns().iter().enumerate() {
            let col_name = column.name().to_string();
            let raw_value = row.try_get_raw(idx)?;
            let value = if raw_value.is_null() {
                serde_json::Value::Null
            } else {
                // Check if this column is an enum (has enum_values in metadata)
                let is_enum = metadata.enum_values.contains_key(&col_name);
                convert_postgres_value_ex(row, idx, column.type_info().name(), is_enum)
            };
            row_map.insert(col_name, value);
        }
        result_rows.push(row_map);
    }

    Ok((columns, column_metadata, result_rows, row_count))
}

/// Process MySQL rows into JSON format with metadata
async fn process_mysql_rows(
    rows: Vec<sqlx::mysql::MySqlRow>,
    metadata: TableMetadata,
) -> AppResult<(Vec<String>, Vec<ColumnMetadata>, Vec<serde_json::Map<String, serde_json::Value>>, usize)> {
    if rows.is_empty() {
        return Ok((vec![], vec![], vec![], 0));
    }

    // Build column metadata from first row
    let (columns, column_metadata): (Vec<String>, Vec<ColumnMetadata>) = rows[0]
        .columns()
        .iter()
        .map(|col| {
            let name = col.name().to_string();
            let data_type = col.type_info().name().to_string();
            (name.clone(), metadata.get_column_metadata(&name, data_type))
        })
        .unzip();

    // Convert rows to JSON with pre-allocated capacity
    let row_count = rows.len();
    let col_count = columns.len();
    let mut result_rows = Vec::with_capacity(row_count);

    for row in &rows {
        let mut row_map = serde_json::Map::with_capacity(col_count);
        for (idx, column) in row.columns().iter().enumerate() {
            let col_name = column.name().to_string();
            let raw_value = row.try_get_raw(idx)?;
            let value = if raw_value.is_null() {
                serde_json::Value::Null
            } else {
                convert_mysql_value(row, idx, column.type_info().name())
            };
            row_map.insert(col_name, value);
        }
        result_rows.push(row_map);
    }

    Ok((columns, column_metadata, result_rows, row_count))
}


/// Convert a PostgreSQL value to JSON based on column type
/// If is_enum is true, the value is from a user-defined enum type
fn convert_postgres_value_ex(row: &sqlx::postgres::PgRow, idx: usize, col_type: &str, is_enum: bool) -> serde_json::Value {
    // Handle enum types first - they need special decoding
    if is_enum {
        // PostgreSQL enums are stored as text internally, try to decode
        // Use try_get_unchecked to bypass type checking for custom enum types
        use sqlx::Row;
        if let Ok(val) = row.try_get_unchecked::<String, _>(idx) {
            return serde_json::Value::String(val);
        }
        // Fallback: try raw value
        if let Ok(raw) = row.try_get_raw(idx) {
            if let Ok(bytes) = <&[u8] as sqlx::Decode<sqlx::Postgres>>::decode(raw) {
                if let Ok(s) = std::str::from_utf8(bytes) {
                    return serde_json::Value::String(s.to_string());
                }
            }
        }
    }

    match col_type {
        "BOOL" => row.try_get::<bool, _>(idx)
            .map(serde_json::Value::Bool)
            .unwrap_or(serde_json::Value::Null),
        "INT2" | "SMALLINT" | "SMALLSERIAL" => row.try_get::<i16, _>(idx)
            .map(|v| serde_json::Value::Number(v.into()))
            .unwrap_or(serde_json::Value::Null),
        "INT4" | "INT" | "SERIAL" => row.try_get::<i32, _>(idx)
            .map(|v| serde_json::Value::Number(v.into()))
            .unwrap_or(serde_json::Value::Null),
        "INT8" | "BIGINT" | "BIGSERIAL" => row.try_get::<i64, _>(idx)
            .map(|v| serde_json::Value::Number(v.into()))
            .unwrap_or(serde_json::Value::Null),
        "FLOAT4" | "REAL" => row.try_get::<f32, _>(idx)
            .map(|v| float_to_json(v as f64))
            .unwrap_or(serde_json::Value::Null),
        "FLOAT8" | "DOUBLE PRECISION" => row.try_get::<f64, _>(idx)
            .map(float_to_json)
            .unwrap_or(serde_json::Value::Null),
        "NUMERIC" | "DECIMAL" => row.try_get::<String, _>(idx)
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        "DATE" => row.try_get::<NaiveDate, _>(idx)
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
        "TIME" => row.try_get::<NaiveTime, _>(idx)
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
        "TIMESTAMP" => row.try_get::<NaiveDateTime, _>(idx)
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
        "TIMESTAMPTZ" | "TIMESTAMP WITH TIME ZONE" => row.try_get::<DateTime<chrono::Utc>, _>(idx)
            .map(|v| serde_json::Value::String(v.to_rfc3339()))
            .unwrap_or(serde_json::Value::Null),
        "UUID" => row.try_get::<uuid::Uuid, _>(idx)
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
        "JSON" | "JSONB" => row.try_get::<serde_json::Value, _>(idx)
            .unwrap_or(serde_json::Value::Null),
        // Network address types
        "INET" | "CIDR" => row.try_get::<IpNetwork, _>(idx)
            .map(|v| serde_json::Value::String(v.to_string()))
            .or_else(|_| row.try_get::<IpAddr, _>(idx).map(|v| serde_json::Value::String(v.to_string())))
            .unwrap_or(serde_json::Value::Null),
        // MAC address types
        "MACADDR" | "MACADDR8" => row.try_get::<[u8; 6], _>(idx)
            .map(|v| serde_json::Value::String(format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                v[0], v[1], v[2], v[3], v[4], v[5])))
            .or_else(|_| row.try_get::<String, _>(idx).map(serde_json::Value::String))
            .unwrap_or(serde_json::Value::Null),
        // Interval type (duration)
        "INTERVAL" => row.try_get::<PgInterval, _>(idx)
            .map(|v| {
                // Format interval as ISO 8601 duration or human-readable
                let total_secs = v.microseconds / 1_000_000;
                let hours = total_secs / 3600;
                let mins = (total_secs % 3600) / 60;
                let secs = total_secs % 60;
                let micros = v.microseconds % 1_000_000;
                if v.months > 0 || v.days > 0 {
                    serde_json::Value::String(format!("{} months {} days {:02}:{:02}:{:02}.{:06}",
                        v.months, v.days, hours, mins, secs, micros))
                } else {
                    serde_json::Value::String(format!("{:02}:{:02}:{:02}.{:06}", hours, mins, secs, micros))
                }
            })
            .or_else(|_| row.try_get::<String, _>(idx).map(serde_json::Value::String))
            .unwrap_or(serde_json::Value::Null),
        // Time with timezone
        "TIMETZ" | "TIME WITH TIME ZONE" => row.try_get::<String, _>(idx)
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        // Money type (returns as string with currency symbol)
        "MONEY" => row.try_get::<String, _>(idx)
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        // Bit string types
        "BIT" | "VARBIT" | "BIT VARYING" => row.try_get::<String, _>(idx)
            .map(serde_json::Value::String)
            .or_else(|_| row.try_get::<Vec<u8>, _>(idx)
                .map(|v| serde_json::Value::String(format!("b'{}'", v.iter().map(|b| format!("{:08b}", b)).collect::<String>()))))
            .unwrap_or(serde_json::Value::Null),
        // Range types (int4range, int8range, daterange, tsrange, tstzrange, numrange)
        "INT4RANGE" | "INT8RANGE" | "DATERANGE" | "TSRANGE" | "TSTZRANGE" | "NUMRANGE" => {
            row.try_get::<String, _>(idx)
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null)
        }
        // OID type (PostgreSQL object identifier)
        "OID" => row.try_get::<Oid, _>(idx)
            .map(|v| serde_json::Value::Number(v.0.into()))
            .or_else(|_| row.try_get::<i64, _>(idx).map(|v| serde_json::Value::Number(v.into())))
            .unwrap_or(serde_json::Value::Null),
        // HSTORE key-value type
        "HSTORE" => row.try_get::<String, _>(idx)
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        // XML type
        "XML" => row.try_get::<String, _>(idx)
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        // CITEXT (case-insensitive text)
        "CITEXT" => row.try_get::<String, _>(idx)
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        // Text types explicitly
        "TEXT" | "VARCHAR" | "CHAR" | "BPCHAR" | "NAME" => row.try_get::<String, _>(idx)
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        // PostgreSQL array types - convert to proper JSON arrays
        "_INT2" => row.try_get::<Vec<i16>, _>(idx)
            .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "_INT4" => row.try_get::<Vec<i32>, _>(idx)
            .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "_INT8" => row.try_get::<Vec<i64>, _>(idx)
            .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "_FLOAT4" => row.try_get::<Vec<f32>, _>(idx)
            .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "_FLOAT8" => row.try_get::<Vec<f64>, _>(idx)
            .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "_TEXT" | "_VARCHAR" | "_BPCHAR" => row.try_get::<Vec<String>, _>(idx)
            .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "_BOOL" => row.try_get::<Vec<bool>, _>(idx)
            .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "_UUID" => row.try_get::<Vec<uuid::Uuid>, _>(idx)
            .map(|v| serde_json::to_value(v.iter().map(|u| u.to_string()).collect::<Vec<_>>()).unwrap_or(serde_json::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "_NUMERIC" => row.try_get::<Vec<rust_decimal::Decimal>, _>(idx)
            .map(|v| serde_json::to_value(v.iter().map(|d| d.to_string()).collect::<Vec<_>>()).unwrap_or(serde_json::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "_DATE" => row.try_get::<Vec<chrono::NaiveDate>, _>(idx)
            .map(|v| serde_json::to_value(v.iter().map(|d| d.to_string()).collect::<Vec<_>>()).unwrap_or(serde_json::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "_TIMESTAMP" => row.try_get::<Vec<chrono::NaiveDateTime>, _>(idx)
            .map(|v| serde_json::to_value(v.iter().map(|d| d.to_string()).collect::<Vec<_>>()).unwrap_or(serde_json::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "_TIMESTAMPTZ" => row.try_get::<Vec<chrono::DateTime<chrono::Utc>>, _>(idx)
            .map(|v| serde_json::to_value(v.iter().map(|d| d.to_rfc3339()).collect::<Vec<_>>()).unwrap_or(serde_json::Value::Null))
            .unwrap_or(serde_json::Value::Null),
        "BYTEA" => row.try_get::<Vec<u8>, _>(idx)
            .map(|bytes| serde_json::Value::String(format!("0x{}", hex::encode(bytes))))
            .unwrap_or(serde_json::Value::Null),
        "GEOMETRY" | "GEOGRAPHY" | "POINT" | "LINESTRING" | "POLYGON" |
        "MULTIPOINT" | "MULTILINESTRING" | "MULTIPOLYGON" | "GEOMETRYCOLLECTION" => {
            if let Ok(wkt) = row.try_get::<String, _>(idx) {
                serde_json::Value::String(wkt)
            } else if let Ok(bytes) = row.try_get::<Vec<u8>, _>(idx) {
                serde_json::Value::String(format!("<PostGIS geometry: {} bytes>", bytes.len()))
            } else {
                serde_json::Value::Null
            }
        }
        _ => {
            if let Ok(val) = row.try_get::<String, _>(idx) {
                serde_json::Value::String(val)
            } else if let Ok(val) = row.try_get::<i64, _>(idx) {
                serde_json::Value::Number(val.into())
            } else if let Ok(val) = row.try_get::<f64, _>(idx) {
                float_to_json(val)
            } else if let Ok(val) = row.try_get::<bool, _>(idx) {
                serde_json::Value::Bool(val)
            } else {
                serde_json::Value::String(format!("<unsupported: {}>", col_type))
            }
        }
    }
}

/// Wrapper for backward compatibility - assumes non-enum column
#[inline]
fn convert_postgres_value(row: &sqlx::postgres::PgRow, idx: usize, col_type: &str) -> serde_json::Value {
    convert_postgres_value_ex(row, idx, col_type, false)
}

/// Convert a MySQL value to JSON based on column type
fn convert_mysql_value(row: &sqlx::mysql::MySqlRow, idx: usize, col_type: &str) -> serde_json::Value {
    match col_type {
        "BOOLEAN" | "TINYINT(1)" => row.try_get::<bool, _>(idx)
            .map(serde_json::Value::Bool)
            .or_else(|_| row.try_get::<i8, _>(idx).map(|v| serde_json::Value::Number(v.into())))
            .unwrap_or(serde_json::Value::Null),
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
        "FLOAT" => row.try_get::<f32, _>(idx)
            .map(|v| float_to_json(v as f64))
            .unwrap_or(serde_json::Value::Null),
        "DOUBLE" | "REAL" => row.try_get::<f64, _>(idx)
            .map(float_to_json)
            .unwrap_or(serde_json::Value::Null),
        "DECIMAL" | "NUMERIC" => row.try_get::<String, _>(idx)
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        "DATE" => row.try_get::<NaiveDate, _>(idx)
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null),
        "TIME" => row.try_get::<NaiveTime, _>(idx)
            .map(|v| serde_json::Value::String(v.to_string()))
            .or_else(|_| row.try_get::<String, _>(idx).map(serde_json::Value::String))
            .unwrap_or(serde_json::Value::Null),
        "DATETIME" | "TIMESTAMP" => row.try_get::<NaiveDateTime, _>(idx)
            .map(|v| serde_json::Value::String(v.to_string()))
            .or_else(|_| row.try_get::<String, _>(idx).map(serde_json::Value::String))
            .unwrap_or(serde_json::Value::Null),
        "YEAR" => row.try_get::<i16, _>(idx)
            .map(|v| serde_json::Value::Number(v.into()))
            .or_else(|_| row.try_get::<String, _>(idx).map(serde_json::Value::String))
            .unwrap_or(serde_json::Value::Null),
        "JSON" => row.try_get::<serde_json::Value, _>(idx)
            .or_else(|_| row.try_get::<String, _>(idx).and_then(|s| {
                serde_json::from_str(&s).map_err(|_| sqlx::Error::ColumnNotFound("json".to_string()))
            }))
            .unwrap_or(serde_json::Value::Null),
        "BINARY" | "VARBINARY" | "TINYBLOB" | "BLOB" | "MEDIUMBLOB" | "LONGBLOB" => {
            row.try_get::<Vec<u8>, _>(idx)
                .map(|bytes| {
                    if bytes.len() > 256 {
                        serde_json::Value::String(format!("0x{}... ({} bytes)", hex::encode(&bytes[..256]), bytes.len()))
                    } else {
                        serde_json::Value::String(format!("0x{}", hex::encode(bytes)))
                    }
                })
                .unwrap_or(serde_json::Value::Null)
        }
        "ENUM" | "SET" => row.try_get::<String, _>(idx)
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        // Text types explicitly
        "CHAR" | "VARCHAR" | "TEXT" | "TINYTEXT" | "MEDIUMTEXT" | "LONGTEXT" => {
            row.try_get::<String, _>(idx)
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null)
        }
        // BIT type - can be 1-64 bits
        "BIT" => row.try_get::<u64, _>(idx)
            .map(|v| serde_json::Value::Number(v.into()))
            .or_else(|_| row.try_get::<Vec<u8>, _>(idx)
                .map(|v| serde_json::Value::String(format!("b'{}'", v.iter().map(|b| format!("{:08b}", b)).collect::<String>()))))
            .or_else(|_| row.try_get::<bool, _>(idx).map(|v| serde_json::Value::Bool(v)))
            .unwrap_or(serde_json::Value::Null),
        // MariaDB VECTOR type (11.7+) - stored as binary, display as array
        "VECTOR" => row.try_get::<Vec<u8>, _>(idx)
            .map(|v| serde_json::Value::String(format!("<vector: {} bytes>", v.len())))
            .or_else(|_| row.try_get::<String, _>(idx).map(serde_json::Value::String))
            .unwrap_or(serde_json::Value::Null),
        "GEOMETRY" | "POINT" | "LINESTRING" | "POLYGON" | "MULTIPOINT" |
        "MULTILINESTRING" | "MULTIPOLYGON" | "GEOMETRYCOLLECTION" => {
            if let Ok(wkt) = row.try_get::<String, _>(idx) {
                serde_json::Value::String(wkt)
            } else if let Ok(bytes) = row.try_get::<Vec<u8>, _>(idx) {
                serde_json::Value::String(format!("<geometry: {} bytes>", bytes.len()))
            } else {
                serde_json::Value::Null
            }
        }
        _ => {
            if let Ok(val) = row.try_get::<String, _>(idx) {
                serde_json::Value::String(val)
            } else if let Ok(val) = row.try_get::<i64, _>(idx) {
                serde_json::Value::Number(val.into())
            } else if let Ok(val) = row.try_get::<f64, _>(idx) {
                float_to_json(val)
            } else if let Ok(val) = row.try_get::<bool, _>(idx) {
                serde_json::Value::Bool(val)
            } else {
                serde_json::Value::String(format!("<unsupported: {}>", col_type))
            }
        }
    }
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

    // Convert rows to JSON using the centralized conversion function
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

            // Use the centralized conversion function
            let value = convert_postgres_value(row, idx, col_type);
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

// Helper function to get enum values for PostgreSQL columns
async fn get_postgres_enum_values(
    pool: &sqlx::PgPool,
    table_name: &str,
    schema_name: &str,
) -> AppResult<HashMap<String, Vec<String>>> {
    // Query to get all enum columns and their values for a table
    let enum_query = r#"
        SELECT
            c.column_name,
            e.enumlabel as enum_value
        FROM information_schema.columns c
        JOIN pg_type t ON c.udt_name = t.typname
        JOIN pg_enum e ON t.oid = e.enumtypid
        WHERE c.table_name = $1
          AND c.table_schema = $2
          AND c.data_type = 'USER-DEFINED'
        ORDER BY c.column_name, e.enumsortorder
    "#;

    let rows = sqlx::query(enum_query)
        .bind(table_name)
        .bind(schema_name)
        .fetch_all(pool)
        .await?;

    let mut enum_map: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let column_name: String = row.try_get("column_name")?;
        let enum_value: String = row.try_get("enum_value")?;
        enum_map
            .entry(column_name)
            .or_default()
            .push(enum_value);
    }

    Ok(enum_map)
}

/// Parse MySQL enum/set definition like enum('val1','val2','escaped''quote')
/// Handles escaped quotes ('') within values
fn parse_mysql_enum_values(column_type: &str) -> Vec<String> {
    let Some(start) = column_type.find('(') else { return vec![] };
    let Some(end) = column_type.rfind(')') else { return vec![] };
    if start >= end { return vec![] }

    let inner = &column_type[start + 1..end];
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut chars = inner.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_quote => in_quote = true,
            '\'' if in_quote => {
                // Check for escaped quote ('')
                if chars.peek() == Some(&'\'') {
                    chars.next();
                    current.push('\'');
                } else {
                    in_quote = false;
                }
            }
            ',' if !in_quote => {
                if !current.is_empty() {
                    values.push(std::mem::take(&mut current));
                }
            }
            _ if in_quote => current.push(c),
            _ => {} // Skip whitespace outside quotes
        }
    }
    if !current.is_empty() {
        values.push(current);
    }
    values
}

async fn get_mysql_enum_values(
    pool: &sqlx::MySqlPool,
    table_name: &str,
    database_name: &str,
) -> AppResult<HashMap<String, Vec<String>>> {
    let rows = sqlx::query(
        "SELECT COLUMN_NAME, COLUMN_TYPE FROM INFORMATION_SCHEMA.COLUMNS \
         WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? AND DATA_TYPE IN ('enum', 'set')"
    )
        .bind(database_name)
        .bind(table_name)
        .fetch_all(pool)
        .await?;

    let mut enum_map = HashMap::with_capacity(rows.len());
    for row in rows {
        let column_name: String = row.try_get("COLUMN_NAME")?;
        let column_type: String = row.try_get("COLUMN_TYPE")?;
        let values = parse_mysql_enum_values(&column_type);
        if !values.is_empty() {
            enum_map.insert(column_name, values);
        }
    }
    Ok(enum_map)
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

    // Convert rows to JSON using the centralized conversion function
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

            // Use the centralized conversion function
            let value = convert_mysql_value(row, idx, col_type);
            row_map.insert(col_name, value);
        }

        result_rows.push(row_map);
    }

    Ok((columns, column_metadata, result_rows, rows.len()))
}
