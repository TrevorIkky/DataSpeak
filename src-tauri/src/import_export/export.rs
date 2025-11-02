use crate::db::connection::{ConnectionManager, DatabaseType};
use crate::error::{AppError, AppResult};
use csv::Writer;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::types::ipnetwork;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportProgress {
    pub table_name: String,
    pub current: usize,
    pub total: usize,
    pub status: String,
    pub cancelled: bool,
}

// Global export cancellation tokens
lazy_static::lazy_static! {
    static ref EXPORT_TOKENS: Arc<RwLock<HashMap<String, CancellationToken>>> = Arc::new(RwLock::new(HashMap::new()));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    pub connection_id: String,
    pub tables: Vec<String>,
    pub output_dir: String,
    pub create_zip: bool,
}

pub async fn export_tables(
    app: AppHandle,
    manager: &ConnectionManager,
    options: ExportOptions,
) -> AppResult<String> {
    use futures::stream::{self, StreamExt};
    use tokio::sync::Mutex;

    // Create and register cancellation token
    let cancel_token = CancellationToken::new();
    let export_id = options.connection_id.clone();
    {
        let mut tokens = EXPORT_TOKENS.write().await;
        tokens.insert(export_id.clone(), cancel_token.clone());
    }

    // Determine paths based on whether we're creating a ZIP
    let (temp_dir, final_path) = if options.create_zip {
        // For ZIP: user selected path is the final ZIP location
        let zip_path = PathBuf::from(&options.output_dir);

        // Create a temporary directory for CSV files
        let parent = zip_path.parent()
            .ok_or_else(|| AppError::IoError("Invalid output path".to_string()))?;
        let temp_dir_name = format!(
            ".dataspeak_export_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        let temp_dir = parent.join(temp_dir_name);
        fs::create_dir_all(&temp_dir).map_err(|e| {
            AppError::IoError(format!("Failed to create temporary directory: {}", e))
        })?;

        (temp_dir, zip_path)
    } else {
        // For CSV: user selected path is the directory for CSV files
        let output_path = PathBuf::from(&options.output_dir);
        fs::create_dir_all(&output_path).map_err(|e| {
            AppError::IoError(format!("Failed to create output directory: {}", e))
        })?;
        (output_path.clone(), output_path)
    };

    let conn = manager.get_connection(&options.connection_id)?;
    let db_type = conn.database_type.clone();
    let table_names = options.tables.clone();
    let total_tables = table_names.len();

    // Emit start event
    app.emit(
        "export-progress",
        ExportProgress {
            table_name: String::new(),
            current: 0,
            total: total_tables,
            status: "Starting export...".to_string(),
            cancelled: false,
        },
    )
    .ok();

    // Use Arc to share progress counter across tasks
    let completed = Arc::new(Mutex::new(0_usize));
    let app_handle = app.clone();
    let connection_id = options.connection_id.clone();

    // Export schema first
    app.emit(
        "export-progress",
        ExportProgress {
            table_name: String::new(),
            current: 0,
            total: total_tables,
            status: "Exporting database schema...".to_string(),
            cancelled: false,
        },
    )
    .ok();

    let schema_path = temp_dir.join("schema.sql");
    export_schema(manager, &connection_id, &schema_path, &db_type, &app).await?;

    // Export tables in parallel with concurrency limit
    let results: Vec<AppResult<()>> = stream::iter(table_names.into_iter())
        .map(|table_name| {
            let connection_id = connection_id.clone();
            let temp_dir = temp_dir.clone();
            let db_type = db_type.clone();
            let completed = completed.clone();
            let app = app_handle.clone();
            let total = total_tables;
            let cancel_token = cancel_token.clone();

            async move {
                // Check for cancellation
                if cancel_token.is_cancelled() {
                    return Err(AppError::OperationCancelled("Export cancelled by user".to_string()));
                }

                // Export the table
                let result = export_table_to_csv(
                    manager,
                    &connection_id,
                    &table_name,
                    &temp_dir,
                    &db_type,
                )
                .await;

                // Update progress
                let mut count = completed.lock().await;
                *count += 1;
                let current = *count;
                drop(count);

                app.emit(
                    "export-progress",
                    ExportProgress {
                        table_name: table_name.clone(),
                        current,
                        total,
                        status: format!("Exported table: {}", table_name),
                        cancelled: false,
                    },
                )
                .ok();

                result
            }
        })
        .buffer_unordered(8) // Process up to 8 tables concurrently
        .collect()
        .await;

    // Check for cancellation or errors
    let mut was_cancelled = false;
    for result in results {
        match result {
            Err(AppError::OperationCancelled(_)) => {
                was_cancelled = true;
                break;
            }
            Err(e) => return Err(e),
            Ok(_) => {}
        }
    }

    // Clean up cancellation token
    {
        let mut tokens = EXPORT_TOKENS.write().await;
        tokens.remove(&export_id);
    }

    if was_cancelled {
        app.emit(
            "export-progress",
            ExportProgress {
                table_name: String::new(),
                current: total_tables,
                total: total_tables,
                status: "Export cancelled".to_string(),
                cancelled: true,
            },
        )
        .ok();
        return Err(AppError::OperationCancelled("Export cancelled by user".to_string()));
    }

    // Create ZIP if requested
    let result_path = if options.create_zip {
        app.emit(
            "export-progress",
            ExportProgress {
                table_name: String::new(),
                current: total_tables,
                total: total_tables,
                status: "Compressing files into ZIP archive...".to_string(),
                cancelled: false,
            },
        )
        .ok();

        // Create ZIP archive at the user-specified location
        create_zip_archive(&temp_dir, &final_path, app.clone(), total_tables)?;

        // Clean up temporary directory
        fs::remove_dir_all(&temp_dir).ok();

        final_path.to_string_lossy().to_string()
    } else {
        final_path.to_string_lossy().to_string()
    };

    // Clean up cancellation token
    {
        let mut tokens = EXPORT_TOKENS.write().await;
        tokens.remove(&export_id);
    }

    // Emit completion event
    app.emit(
        "export-progress",
        ExportProgress {
            table_name: String::new(),
            current: total_tables,
            total: total_tables,
            status: "Export completed!".to_string(),
            cancelled: false,
        },
    )
    .ok();

    Ok(result_path)
}

/// Cancel an ongoing export operation
pub async fn cancel_export(connection_id: String) -> AppResult<()> {
    let tokens = EXPORT_TOKENS.read().await;
    if let Some(token) = tokens.get(&connection_id) {
        token.cancel();
        Ok(())
    } else {
        Err(AppError::Other("No active export found for this connection".to_string()))
    }
}

async fn export_table_to_csv(
    manager: &ConnectionManager,
    connection_id: &str,
    table_name: &str,
    output_path: &PathBuf,
    db_type: &DatabaseType,
) -> AppResult<()> {
    match db_type {
        DatabaseType::PostgreSQL => {
            export_postgres_table(manager, connection_id, table_name, output_path).await
        }
        DatabaseType::MariaDB | DatabaseType::MySQL => {
            export_mysql_table(manager, connection_id, table_name, output_path).await
        }
    }
}

async fn export_postgres_table(
    manager: &ConnectionManager,
    connection_id: &str,
    table_name: &str,
    output_path: &PathBuf,
) -> AppResult<()> {
    let pool = manager.get_pool_postgres(connection_id).await?;

    // First, query column metadata to get types
    let type_query = format!(
        "SELECT column_name, udt_name, data_type
         FROM information_schema.columns
         WHERE table_name = '{}' AND table_schema = 'public'
         ORDER BY ordinal_position",
        table_name
    );

    let column_metadata: Vec<(String, String, String)> = sqlx::query_as(&type_query)
        .fetch_all(&pool)
        .await?;

    if column_metadata.is_empty() {
        return Err(AppError::DatabaseError(format!("Table '{}' not found or has no columns", table_name)));
    }

    // Build SELECT query with special handling for geometry/geography types
    let select_parts: Vec<String> = column_metadata
        .iter()
        .map(|(column_name, udt_name, _)| {
            match udt_name.as_str() {
                "geometry" | "geography" => {
                    // Export geometry as EWKT (includes SRID)
                    format!("ST_AsEWKT(\"{}\") as \"{}\"", column_name, column_name)
                }
                _ => format!("\"{}\"", column_name)
            }
        })
        .collect();

    let query = format!("SELECT {} FROM \"{}\"", select_parts.join(", "), table_name);
    let rows = sqlx::query(&query).fetch_all(&pool).await?;

    let csv_path = output_path.join(format!("{}.csv", table_name));
    let file = File::create(&csv_path).map_err(|e| {
        AppError::IoError(format!("Failed to create CSV file: {}", e))
    })?;

    let mut writer = Writer::from_writer(file);

    // Write header
    let column_names: Vec<String> = column_metadata.iter().map(|(name, _, _)| name.clone()).collect();
    writer.write_record(&column_names).map_err(|e| {
        AppError::IoError(format!("Failed to write CSV header: {}", e))
    })?;

    if rows.is_empty() {
        writer.flush().map_err(|e| {
            AppError::IoError(format!("Failed to flush CSV: {}", e))
        })?;
        return Ok(());
    }

    // Convert rows to records using rayon for parallel processing
    // Use column metadata to determine how to format each value
    let csv_records: Vec<Vec<String>> = rows
        .par_iter()
        .map(|row| {
            column_metadata
                .iter()
                .enumerate()
                .map(|(idx, (_, udt_name, data_type))| {
                    format_postgres_value(row, idx, udt_name, data_type)
                })
                .collect()
        })
        .collect();

    // Write all records (csv crate handles escaping automatically)
    for record in csv_records {
        writer.write_record(&record).map_err(|e| {
            AppError::IoError(format!("Failed to write CSV row: {}", e))
        })?;
    }

    writer.flush().map_err(|e| {
        AppError::IoError(format!("Failed to flush CSV: {}", e))
    })?;

    Ok(())
}

/// Format a PostgreSQL value based on its type
fn format_postgres_value(
    row: &sqlx::postgres::PgRow,
    idx: usize,
    udt_name: &str,
    data_type: &str,
) -> String {
    use sqlx::Row;

    match udt_name {
        // UUID type
        "uuid" => {
            if let Ok(val) = row.try_get::<Option<uuid::Uuid>, _>(idx) {
                return val.map(|v| v.to_string()).unwrap_or_default();
            }
        }

        // Numeric/Decimal types (arbitrary precision)
        "numeric" => {
            if let Ok(val) = row.try_get::<Option<rust_decimal::Decimal>, _>(idx) {
                return val.map(|v| v.to_string()).unwrap_or_default();
            }
        }

        // JSON/JSONB types
        "json" | "jsonb" => {
            if let Ok(val) = row.try_get::<Option<serde_json::Value>, _>(idx) {
                return val.map(|v| v.to_string()).unwrap_or_default();
            }
        }

        // IP address types (inet, cidr)
        "inet" | "cidr" => {
            if let Ok(val) = row.try_get::<Option<ipnetwork::IpNetwork>, _>(idx) {
                return val.map(|v| v.to_string()).unwrap_or_default();
            }
            // Fallback to IpAddr for simple inet
            if let Ok(val) = row.try_get::<Option<std::net::IpAddr>, _>(idx) {
                return val.map(|v| v.to_string()).unwrap_or_default();
            }
        }

        // MAC address type
        "macaddr" | "macaddr8" => {
            if let Ok(val) = row.try_get::<Option<[u8; 6]>, _>(idx) {
                return val.map(|v| {
                    format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                        v[0], v[1], v[2], v[3], v[4], v[5])
                }).unwrap_or_default();
            }
        }

        // Interval type
        "interval" => {
            if let Ok(val) = row.try_get::<Option<sqlx::postgres::types::PgInterval>, _>(idx) {
                return val.map(|v| {
                    // Format as PostgreSQL interval string
                    let mut parts = Vec::new();
                    if v.months != 0 {
                        parts.push(format!("{} months", v.months));
                    }
                    if v.days != 0 {
                        parts.push(format!("{} days", v.days));
                    }
                    if v.microseconds != 0 {
                        let seconds = v.microseconds as f64 / 1_000_000.0;
                        parts.push(format!("{} seconds", seconds));
                    }
                    if parts.is_empty() {
                        "0".to_string()
                    } else {
                        parts.join(" ")
                    }
                }).unwrap_or_default();
            }
        }

        // Array types
        "_int4" | "_int8" | "_int2" => {
            // Integer arrays
            if let Ok(val) = row.try_get::<Option<Vec<i32>>, _>(idx) {
                return val.map(|v| format!("{{{}}}", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))).unwrap_or_default();
            }
            if let Ok(val) = row.try_get::<Option<Vec<i64>>, _>(idx) {
                return val.map(|v| format!("{{{}}}", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))).unwrap_or_default();
            }
            if let Ok(val) = row.try_get::<Option<Vec<i16>>, _>(idx) {
                return val.map(|v| format!("{{{}}}", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))).unwrap_or_default();
            }
        }

        "_text" | "_varchar" => {
            // Text/String arrays
            if let Ok(val) = row.try_get::<Option<Vec<String>>, _>(idx) {
                return val.map(|v| {
                    // Escape strings in array and quote them
                    let escaped: Vec<String> = v.iter().map(|s| {
                        if s.contains(',') || s.contains('{') || s.contains('}') || s.contains('"') || s.contains('\\') {
                            format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
                        } else {
                            s.clone()
                        }
                    }).collect();
                    format!("{{{}}}", escaped.join(","))
                }).unwrap_or_default();
            }
        }

        "_bool" => {
            // Boolean arrays
            if let Ok(val) = row.try_get::<Option<Vec<bool>>, _>(idx) {
                return val.map(|v| format!("{{{}}}", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))).unwrap_or_default();
            }
        }

        // Geometry/Geography types (already converted to EWKT in SELECT)
        "geometry" | "geography" => {
            if let Ok(val) = row.try_get::<Option<String>, _>(idx) {
                return val.unwrap_or_default();
            }
        }

        // Binary types
        "bytea" => {
            if let Ok(val) = row.try_get::<Option<Vec<u8>>, _>(idx) {
                return val.map(|v| format!("\\x{}", hex::encode(v))).unwrap_or_default();
            }
        }

        _ => {}
    }

    // Generic type handling based on data_type
    match data_type {
        "ARRAY" => {
            // Generic array fallback - try as string array
            if let Ok(val) = row.try_get::<Option<Vec<String>>, _>(idx) {
                return val.map(|v| format!("{{{}}}", v.join(","))).unwrap_or_default();
            }
        }
        _ => {}
    }

    // Standard types - try in order of likelihood
    // String types (most common)
    if let Ok(val) = row.try_get::<Option<String>, _>(idx) {
        return val.unwrap_or_default();
    }

    // DateTime types
    if let Ok(val) = row.try_get::<Option<chrono::NaiveDateTime>, _>(idx) {
        return val.map(|v| v.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(idx) {
        return val.map(|v| v.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<chrono::NaiveDate>, _>(idx) {
        return val.map(|v| v.format("%Y-%m-%d").to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<chrono::NaiveTime>, _>(idx) {
        return val.map(|v| v.format("%H:%M:%S").to_string()).unwrap_or_default();
    }

    // Integer types
    if let Ok(val) = row.try_get::<Option<i16>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<i32>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<i64>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }

    // Float types
    if let Ok(val) = row.try_get::<Option<f32>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<f64>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }

    // Boolean
    if let Ok(val) = row.try_get::<Option<bool>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }

    // Binary data fallback
    if let Ok(val) = row.try_get::<Option<Vec<u8>>, _>(idx) {
        return val.map(|v| format!("\\x{}", hex::encode(v))).unwrap_or_default();
    }

    // Fallback for unknown types
    String::new()
}

async fn export_mysql_table(
    manager: &ConnectionManager,
    connection_id: &str,
    table_name: &str,
    output_path: &PathBuf,
) -> AppResult<()> {
    let pool = manager.get_pool_mysql(connection_id).await?;

    // First, query column metadata to get types
    let type_query = format!(
        "SELECT COLUMN_NAME, DATA_TYPE, COLUMN_TYPE
         FROM INFORMATION_SCHEMA.COLUMNS
         WHERE TABLE_NAME = '{}' AND TABLE_SCHEMA = DATABASE()
         ORDER BY ORDINAL_POSITION",
        table_name
    );

    let column_metadata: Vec<(String, String, String)> = sqlx::query_as(&type_query)
        .fetch_all(&pool)
        .await?;

    if column_metadata.is_empty() {
        return Err(AppError::DatabaseError(format!("Table '{}' not found or has no columns", table_name)));
    }

    // Get all rows
    let query = format!("SELECT * FROM `{}`", table_name);
    let rows = sqlx::query(&query).fetch_all(&pool).await?;

    let csv_path = output_path.join(format!("{}.csv", table_name));
    let file = File::create(&csv_path).map_err(|e| {
        AppError::IoError(format!("Failed to create CSV file: {}", e))
    })?;

    let mut writer = Writer::from_writer(file);

    // Write header
    let column_names: Vec<String> = column_metadata.iter().map(|(name, _, _)| name.clone()).collect();
    writer.write_record(&column_names).map_err(|e| {
        AppError::IoError(format!("Failed to write CSV header: {}", e))
    })?;

    if rows.is_empty() {
        writer.flush().map_err(|e| {
            AppError::IoError(format!("Failed to flush CSV: {}", e))
        })?;
        return Ok(());
    }

    // Convert rows to records using rayon for parallel processing
    // Use column metadata to determine how to format each value
    let csv_records: Vec<Vec<String>> = rows
        .par_iter()
        .map(|row| {
            column_metadata
                .iter()
                .enumerate()
                .map(|(idx, (_, data_type, column_type))| {
                    format_mysql_value(row, idx, data_type, column_type)
                })
                .collect()
        })
        .collect();

    // Write all records (csv crate handles escaping automatically)
    for record in csv_records {
        writer.write_record(&record).map_err(|e| {
            AppError::IoError(format!("Failed to write CSV row: {}", e))
        })?;
    }

    writer.flush().map_err(|e| {
        AppError::IoError(format!("Failed to flush CSV: {}", e))
    })?;

    Ok(())
}

/// Format a MySQL/MariaDB value based on its type
fn format_mysql_value(
    row: &sqlx::mysql::MySqlRow,
    idx: usize,
    data_type: &str,
    _column_type: &str,
) -> String {
    use sqlx::Row;

    // Handle specific data types
    let data_type_lower = data_type.to_lowercase();

    match data_type_lower.as_str() {
        // JSON type
        "json" => {
            if let Ok(val) = row.try_get::<Option<serde_json::Value>, _>(idx) {
                return val.map(|v| v.to_string()).unwrap_or_default();
            }
        }

        // Decimal/Numeric types (arbitrary precision)
        "decimal" | "numeric" => {
            if let Ok(val) = row.try_get::<Option<rust_decimal::Decimal>, _>(idx) {
                return val.map(|v| v.to_string()).unwrap_or_default();
            }
        }

        // Geometry types (MySQL spatial data)
        "geometry" | "point" | "linestring" | "polygon" |
        "multipoint" | "multilinestring" | "multipolygon" | "geometrycollection" => {
            // MySQL returns geometry as binary, convert to WKT for portability
            if let Ok(val) = row.try_get::<Option<Vec<u8>>, _>(idx) {
                return val.map(|v| format!("0x{}", hex::encode(v))).unwrap_or_default();
            }
        }

        // Binary types
        "binary" | "varbinary" | "blob" | "tinyblob" | "mediumblob" | "longblob" => {
            if let Ok(val) = row.try_get::<Option<Vec<u8>>, _>(idx) {
                return val.map(|v| format!("0x{}", hex::encode(v))).unwrap_or_default();
            }
        }

        // Bit type
        "bit" => {
            // Try as u64 first for BIT columns
            if let Ok(val) = row.try_get::<Option<u64>, _>(idx) {
                return val.map(|v| v.to_string()).unwrap_or_default();
            }
        }

        // Set and Enum types are returned as strings by MySQL
        "set" | "enum" => {
            if let Ok(val) = row.try_get::<Option<String>, _>(idx) {
                return val.unwrap_or_default();
            }
        }

        _ => {}
    }

    // Standard types - try in order of likelihood
    // String types (most common)
    if let Ok(val) = row.try_get::<Option<String>, _>(idx) {
        return val.unwrap_or_default();
    }

    // DateTime types
    if let Ok(val) = row.try_get::<Option<chrono::NaiveDateTime>, _>(idx) {
        return val.map(|v| v.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(idx) {
        return val.map(|v| v.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<chrono::NaiveDate>, _>(idx) {
        return val.map(|v| v.format("%Y-%m-%d").to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<chrono::NaiveTime>, _>(idx) {
        return val.map(|v| v.format("%H:%M:%S").to_string()).unwrap_or_default();
    }

    // Signed integer types
    if let Ok(val) = row.try_get::<Option<i8>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<i16>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<i32>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<i64>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }

    // Unsigned integer types
    if let Ok(val) = row.try_get::<Option<u8>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<u16>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<u32>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<u64>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }

    // Float types
    if let Ok(val) = row.try_get::<Option<f32>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }
    if let Ok(val) = row.try_get::<Option<f64>, _>(idx) {
        return val.map(|v| v.to_string()).unwrap_or_default();
    }

    // Boolean (TINYINT(1) in MySQL)
    if let Ok(val) = row.try_get::<Option<bool>, _>(idx) {
        return val.map(|v| if v { "1".to_string() } else { "0".to_string() }).unwrap_or_default();
    }

    // Binary data fallback
    if let Ok(val) = row.try_get::<Option<Vec<u8>>, _>(idx) {
        return val.map(|v| format!("0x{}", hex::encode(v))).unwrap_or_default();
    }

    // Fallback for unknown types
    String::new()
}

fn create_zip_archive(
    source_dir: &PathBuf,
    zip_path: &PathBuf,
    app: AppHandle,
    total_tables: usize,
) -> AppResult<()> {
    use std::io::{BufReader, BufWriter, Read};
    use zip::write::FileOptions;
    use zip::CompressionMethod;

    // Use buffered writer for better I/O performance
    let file = File::create(zip_path).map_err(|e| {
        AppError::IoError(format!("Failed to create ZIP file: {}", e))
    })?;
    let buffered_file = BufWriter::with_capacity(256 * 1024, file); // 256KB buffer

    let mut zip = zip::ZipWriter::new(buffered_file);

    // Use Stored (no compression) for CSV files as they're mostly text
    // This is much faster and CSV data doesn't compress well anyway
    let options: FileOptions<()> = FileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o755);

    // Collect all CSV and SQL files first to show progress
    let entries: Vec<_> = fs::read_dir(source_dir)
        .map_err(|e| AppError::IoError(format!("Failed to read directory: {}", e)))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let path = entry.path();
            let ext = path.extension().and_then(|s| s.to_str());
            ext == Some("csv") || ext == Some("sql")
        })
        .collect();

    let total_files = entries.len();

    // Add all CSV and SQL files to the ZIP archive with streaming I/O
    for (idx, entry) in entries.iter().enumerate() {
        let path = entry.path();
        let file_name = path
            .file_name()
            .ok_or_else(|| AppError::IoError("Invalid file name".to_string()))?
            .to_string_lossy()
            .to_string();

        // Emit progress for each file being compressed
        let display_name = if file_name.ends_with(".sql") {
            "schema".to_string()
        } else {
            file_name.replace(".csv", "")
        };

        app.emit(
            "export-progress",
            ExportProgress {
                table_name: display_name,
                current: total_tables,
                total: total_tables,
                status: format!("Adding {} to archive ({}/{})", file_name, idx + 1, total_files),
                cancelled: false,
            },
        )
        .ok();

        zip.start_file(&file_name, options).map_err(|e| {
            AppError::IoError(format!("Failed to start ZIP file entry: {}", e))
        })?;

        // Use streaming I/O with a large buffer instead of reading entire file
        let f = File::open(&path).map_err(|e| {
            AppError::IoError(format!("Failed to open file: {}", e))
        })?;
        let mut reader = BufReader::with_capacity(256 * 1024, f); // 256KB read buffer

        // Stream the file in chunks
        let mut chunk = [0u8; 256 * 1024]; // 256KB chunks
        loop {
            let bytes_read = reader.read(&mut chunk).map_err(|e| {
                AppError::IoError(format!("Failed to read file: {}", e))
            })?;

            if bytes_read == 0 {
                break;
            }

            zip.write_all(&chunk[..bytes_read]).map_err(|e| {
                AppError::IoError(format!("Failed to write to ZIP: {}", e))
            })?;
        }
    }

    // Emit finalizing status
    app.emit(
        "export-progress",
        ExportProgress {
            table_name: String::new(),
            current: total_tables,
            total: total_tables,
            status: "Finalizing ZIP archive...".to_string(),
            cancelled: false,
        },
    )
    .ok();

    zip.finish().map_err(|e| {
        AppError::IoError(format!("Failed to finalize ZIP: {}", e))
    })?;

    Ok(())
}

/// Export database schema to schema.sql file
async fn export_schema(
    manager: &ConnectionManager,
    connection_id: &str,
    output_path: &PathBuf,
    db_type: &DatabaseType,
    app: &AppHandle,
) -> AppResult<()> {
    match db_type {
        DatabaseType::PostgreSQL => {
            export_postgres_schema(manager, connection_id, output_path, app).await
        }
        DatabaseType::MariaDB | DatabaseType::MySQL => {
            export_mysql_schema(manager, connection_id, output_path, app).await
        }
    }
}

/// Export PostgreSQL schema using optimized bulk queries
async fn export_postgres_schema(
    manager: &ConnectionManager,
    connection_id: &str,
    output_path: &PathBuf,
    app: &AppHandle,
) -> AppResult<()> {
    let pool = manager.get_pool_postgres(connection_id).await?;

    app.emit(
        "export-progress",
        ExportProgress {
            table_name: String::new(),
            current: 0,
            total: 2,
            status: "Fetching schema definitions...".to_string(),
            cancelled: false,
        },
    )
    .ok();

    let mut file = BufWriter::new(File::create(output_path).map_err(|e| {
        AppError::IoError(format!("Failed to create schema file: {}", e))
    })?);

    writeln!(file, "-- PostgreSQL Database Schema").map_err(|e| {
        AppError::IoError(format!("Failed to write to schema file: {}", e))
    })?;
    writeln!(file, "-- Generated by DataSpeak\n").map_err(|e| {
        AppError::IoError(format!("Failed to write to schema file: {}", e))
    })?;

    // Fetch table definitions and constraints in parallel for maximum speed
    let tables_future = sqlx::query_as::<_, (String,)>(
        r#"
        WITH table_columns AS MATERIALIZED (
            SELECT
                a.attrelid,
                a.attnum,
                a.attname,
                format_type(a.atttypid, a.atttypmod) as data_type,
                a.attnotnull,
                pg_get_expr(ad.adbin, ad.adrelid) as default_expr
            FROM pg_attribute a
            LEFT JOIN pg_attrdef ad ON ad.adrelid = a.attrelid AND ad.adnum = a.attnum
            WHERE a.attnum > 0 AND NOT a.attisdropped
        )
        SELECT
            'CREATE TABLE "' || c.relname || '" (' ||
            string_agg(
                '"' || tc.attname || '" ' || tc.data_type ||
                CASE WHEN tc.attnotnull THEN ' NOT NULL' ELSE '' END ||
                CASE WHEN tc.default_expr IS NOT NULL THEN ' DEFAULT ' || tc.default_expr ELSE '' END,
                ', ' ORDER BY tc.attnum
            ) || ');' as create_stmt
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        JOIN table_columns tc ON tc.attrelid = c.oid
        WHERE c.relkind = 'r' AND n.nspname = 'public'
        GROUP BY c.oid, c.relname
        ORDER BY c.relname
        "#
    )
    .fetch_all(&pool);

    let constraints_future = sqlx::query_as::<_, (String,)>(
        r#"
        SELECT
            'ALTER TABLE "' || c.relname || '" ADD CONSTRAINT ' || con.conname || ' ' ||
            pg_get_constraintdef(con.oid) || ';' as constraint_stmt
        FROM pg_constraint con
        JOIN pg_class c ON c.oid = con.conrelid
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public'
            AND con.contype IN ('p', 'f')
        ORDER BY c.relname, con.contype DESC
        "#
    )
    .fetch_all(&pool);

    // Execute both queries in parallel
    let (schema_sql, constraints) = tokio::join!(tables_future, constraints_future);
    let schema_sql = schema_sql?;
    let constraints = constraints?;

    app.emit(
        "export-progress",
        ExportProgress {
            table_name: String::new(),
            current: 1,
            total: 2,
            status: "Writing schema to file...".to_string(),
            cancelled: false,
        },
    )
    .ok();

    // Write DROP TABLE IF EXISTS and CREATE TABLE statements
    for (create_stmt,) in schema_sql {
        // Extract table name from CREATE TABLE statement
        // Format: CREATE TABLE "table_name" (...)
        if let Some(table_name) = create_stmt
            .strip_prefix("CREATE TABLE \"")
            .and_then(|s| s.split('"').next())
        {
            // Write DROP TABLE IF EXISTS first for idempotent imports
            writeln!(file, "DROP TABLE IF EXISTS \"{}\" CASCADE;\n", table_name).map_err(|e| {
                AppError::IoError(format!("Failed to write to schema file: {}", e))
            })?;
        }

        // Write CREATE TABLE statement
        writeln!(file, "{}\n", create_stmt).map_err(|e| {
            AppError::IoError(format!("Failed to write to schema file: {}", e))
        })?;
    }

    // Write all constraints
    for (constraint_stmt,) in constraints {
        writeln!(file, "{}\n", constraint_stmt).map_err(|e| {
            AppError::IoError(format!("Failed to write to schema file: {}", e))
        })?;
    }

    app.emit(
        "export-progress",
        ExportProgress {
            table_name: String::new(),
            current: 3,
            total: 3,
            status: "Schema export complete".to_string(),
            cancelled: false,
        },
    )
    .ok();

    file.flush().map_err(|e| {
        AppError::IoError(format!("Failed to flush schema file: {}", e))
    })?;

    Ok(())
}

/// Export MySQL/MariaDB schema using parallel SHOW CREATE TABLE
async fn export_mysql_schema(
    manager: &ConnectionManager,
    connection_id: &str,
    output_path: &PathBuf,
    app: &AppHandle,
) -> AppResult<()> {
    use futures::stream::{self, StreamExt};

    let pool = manager.get_pool_mysql(connection_id).await?;

    app.emit(
        "export-progress",
        ExportProgress {
            table_name: String::new(),
            current: 0,
            total: 1,
            status: "Fetching table list...".to_string(),
            cancelled: false,
        },
    )
    .ok();

    // Get all tables
    let tables: Vec<(String,)> = sqlx::query_as("SHOW TABLES")
        .fetch_all(&pool)
        .await?;

    let total_tables = tables.len();

    app.emit(
        "export-progress",
        ExportProgress {
            table_name: String::new(),
            current: 0,
            total: total_tables,
            status: "Fetching table schemas ...".to_string(),
            cancelled: false,
        },
    )
    .ok();

    // Fetch all table schemas in parallel (up to 16 concurrent)
    let schema_results: Vec<AppResult<(String, String)>> = stream::iter(tables.into_iter())
        .map(|(table_name,)| {
            let pool = pool.clone();
            async move {
                let create_result: (String, String) =
                    sqlx::query_as(&format!("SHOW CREATE TABLE `{}`", table_name))
                        .fetch_one(&pool)
                        .await?;
                Ok((table_name, create_result.1))
            }
        })
        .buffer_unordered(16) // Process up to 16 tables concurrently
        .collect()
        .await;

    app.emit(
        "export-progress",
        ExportProgress {
            table_name: String::new(),
            current: 1,
            total: 2,
            status: "Writing schema to file...".to_string(),
            cancelled: false,
        },
    )
    .ok();

    let mut file = BufWriter::new(File::create(output_path).map_err(|e| {
        AppError::IoError(format!("Failed to create schema file: {}", e))
    })?);

    writeln!(file, "-- MySQL/MariaDB Database Schema").map_err(|e| {
        AppError::IoError(format!("Failed to write to schema file: {}", e))
    })?;
    writeln!(file, "-- Generated by DataSpeak\n").map_err(|e| {
        AppError::IoError(format!("Failed to write to schema file: {}", e))
    })?;

    // Collect and sort results by table name for consistent output
    let mut schemas: Vec<(String, String)> = Vec::new();
    for result in schema_results {
        schemas.push(result?);
    }
    schemas.sort_by(|a, b| a.0.cmp(&b.0));

    // Write DROP TABLE IF EXISTS and CREATE TABLE statements in order
    for (table_name, create_stmt) in schemas {
        // Write DROP TABLE IF EXISTS first for idempotent imports
        writeln!(file, "DROP TABLE IF EXISTS `{}`;\n", table_name).map_err(|e| {
            AppError::IoError(format!("Failed to write to schema file: {}", e))
        })?;

        // Write CREATE TABLE statement
        writeln!(file, "{};\n", create_stmt).map_err(|e| {
            AppError::IoError(format!("Failed to write to schema file: {}", e))
        })?;
    }

    app.emit(
        "export-progress",
        ExportProgress {
            table_name: String::new(),
            current: 2,
            total: 2,
            status: "Schema export complete".to_string(),
            cancelled: false,
        },
    )
    .ok();

    file.flush().map_err(|e| {
        AppError::IoError(format!("Failed to flush schema file: {}", e))
    })?;

    Ok(())
}
