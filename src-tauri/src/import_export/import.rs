use crate::db::connection::{ConnectionManager, DatabaseType};
use crate::error::{AppError, AppResult};
use crate::import_export::export::CSV_NULL_MARKER;
use csv::ReaderBuilder;
use futures::stream::{self, StreamExt};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use sqlparser::dialect::{MySqlDialect, PostgreSqlDialect};
use sqlparser::parser::Parser;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

/// Safely quote a PostgreSQL identifier (table/column name)
fn quote_identifier_postgres(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

/// Safely quote a MySQL identifier (table/column name)
fn quote_identifier_mysql(identifier: &str) -> String {
    format!("`{}`", identifier.replace('`', "``"))
}

/// Validate schema SQL to prevent malicious statements
/// Only allows: CREATE TABLE, DROP TABLE IF EXISTS, ALTER TABLE, CREATE INDEX
fn validate_schema_sql(sql: &str, db_type: &DatabaseType) -> AppResult<()> {
    use sqlparser::ast::Statement;

    let statements = match db_type {
        DatabaseType::PostgreSQL => {
            Parser::parse_sql(&PostgreSqlDialect {}, sql).map_err(|e| {
                AppError::ValidationError(format!("Invalid SQL syntax: {}", e))
            })?
        }
        DatabaseType::MariaDB | DatabaseType::MySQL => {
            Parser::parse_sql(&MySqlDialect {}, sql).map_err(|e| {
                AppError::ValidationError(format!("Invalid SQL syntax: {}", e))
            })?
        }
    };

    for stmt in statements {
        let allowed = matches!(
            stmt,
            Statement::CreateTable { .. }
                | Statement::CreateIndex { .. }
                | Statement::AlterTable { .. }
                | Statement::Drop { .. }
        );

        if !allowed {
            return Err(AppError::ValidationError(format!(
                "Schema file contains disallowed statement type. Only CREATE TABLE, DROP TABLE, ALTER TABLE, and CREATE INDEX are allowed. Found: {}",
                stmt.to_string().chars().take(50).collect::<String>()
            )));
        }

        // Extra check for DROP: only allow DROP TABLE, not DROP DATABASE
        if let Statement::Drop { object_type, .. } = &stmt {
            let object_str = format!("{:?}", object_type);
            if !object_str.contains("Table") {
                return Err(AppError::ValidationError(format!(
                    "Only DROP TABLE is allowed, found DROP {:?}",
                    object_type
                )));
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub file_name: String,
    pub current: usize,
    pub total: usize,
    pub status: String,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOptions {
    pub connection_id: String,
    pub source_path: String,
    pub is_zip: bool,
    pub table_mappings: HashMap<String, String>, // CSV filename -> table name
}

// Global import cancellation tokens
lazy_static! {
    static ref IMPORT_TOKENS: Arc<RwLock<HashMap<String, CancellationToken>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

pub async fn import_tables(
    app: AppHandle,
    manager: &ConnectionManager,
    options: ImportOptions,
) -> AppResult<()> {
    // Create and register cancellation token
    let cancel_token = CancellationToken::new();
    let import_id = options.connection_id.clone();
    {
        let mut tokens = IMPORT_TOKENS.write().await;
        tokens.insert(import_id.clone(), cancel_token.clone());
    }

    let conn = manager.get_connection(&options.connection_id)?;
    let db_type = conn.database_type.clone();

    // Extract files if ZIP
    let (csv_files, temp_dir) = if options.is_zip {
        app.emit(
            "import-progress",
            ImportProgress {
                file_name: String::new(),
                current: 0,
                total: 1,
                status: "Extracting ZIP archive...".to_string(),
                cancelled: false,
            },
        )
        .ok();

        let (files, dir) = extract_zip_archive_streaming(&options.source_path)?;
        (files, Some(dir))
    } else {
        (vec![PathBuf::from(&options.source_path)], None)
    };

    let total_files = csv_files.len();

    // Import schema first if it exists
    if let Some(ref temp) = temp_dir {
        let schema_path = temp.join("schema.sql");
        if schema_path.exists() {
            app.emit(
                "import-progress",
                ImportProgress {
                    file_name: String::new(),
                    current: 0,
                    total: total_files,
                    status: "Importing database schema...".to_string(),
                    cancelled: false,
                },
            )
            .ok();

            import_schema(manager, &options.connection_id, &schema_path, &db_type).await?;
        } else {
            // No schema.sql found - this might cause issues if tables don't exist
            app.emit(
                "import-progress",
                ImportProgress {
                    file_name: String::new(),
                    current: 0,
                    total: total_files,
                    status: "Warning: No schema.sql found in ZIP. Tables must already exist.".to_string(),
                    cancelled: false,
                },
            )
            .ok();
        }
    }

    // Use Arc to share progress counter across tasks
    let completed = Arc::new(Mutex::new(0_usize));
    let app_handle = app.clone();
    let connection_id = options.connection_id.clone();

    // Import CSV files in parallel (up to 8 concurrent)
    let results: Vec<AppResult<()>> = stream::iter(csv_files.into_iter())
        .map(|csv_path| {
            let connection_id = connection_id.clone();
            let table_mappings = options.table_mappings.clone();
            let db_type = db_type.clone();
            let completed = completed.clone();
            let app = app_handle.clone();
            let total = total_files;
            let cancel_token = cancel_token.clone();

            async move {
                // Check for cancellation
                if cancel_token.is_cancelled() {
                    return Err(AppError::OperationCancelled(
                        "Import cancelled by user".to_string(),
                    ));
                }

                let file_name = csv_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");

                let table_name = table_mappings
                    .get(file_name)
                    .cloned()
                    .unwrap_or_else(|| file_name.to_string());

                // Update progress
                let mut count = completed.lock().await;
                *count += 1;
                let current = *count;
                drop(count);

                app.emit(
                    "import-progress",
                    ImportProgress {
                        file_name: file_name.to_string(),
                        current,
                        total,
                        status: format!("Importing into table: {}", table_name),
                        cancelled: false,
                    },
                )
                .ok();

                // Import CSV with streaming
                import_csv_to_table_streaming(
                    manager,
                    &connection_id,
                    &csv_path,
                    &table_name,
                    &db_type,
                )
                .await
            }
        })
        .buffer_unordered(8) // Process up to 8 files concurrently
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
            Err(e) => {
                // Clean up before returning error
                if let Some(dir) = temp_dir {
                    fs::remove_dir_all(&dir).ok();
                }
                return Err(e);
            }
            Ok(_) => {}
        }
    }

    // Clean up temporary directory
    if let Some(dir) = temp_dir {
        fs::remove_dir_all(&dir).ok();
    }

    // Clean up cancellation token
    {
        let mut tokens = IMPORT_TOKENS.write().await;
        tokens.remove(&import_id);
    }

    if was_cancelled {
        app.emit(
            "import-progress",
            ImportProgress {
                file_name: String::new(),
                current: total_files,
                total: total_files,
                status: "Import cancelled".to_string(),
                cancelled: true,
            },
        )
        .ok();
        return Err(AppError::OperationCancelled(
            "Import cancelled by user".to_string(),
        ));
    }

    // Emit completion event
    app.emit(
        "import-progress",
        ImportProgress {
            file_name: String::new(),
            current: total_files,
            total: total_files,
            status: "Import completed!".to_string(),
            cancelled: false,
        },
    )
    .ok();

    Ok(())
}

/// Cancel an ongoing import operation
pub async fn cancel_import(connection_id: String) -> AppResult<()> {
    let tokens = IMPORT_TOKENS.read().await;
    if let Some(token) = tokens.get(&connection_id) {
        token.cancel();
        Ok(())
    } else {
        Err(AppError::Other(
            "No active import found for this connection".to_string(),
        ))
    }
}

/// Import schema.sql file
async fn import_schema(
    manager: &ConnectionManager,
    connection_id: &str,
    schema_path: &PathBuf,
    db_type: &DatabaseType,
) -> AppResult<()> {
    let schema_content = fs::read_to_string(schema_path).map_err(|e| {
        AppError::IoError(format!("Failed to read schema file: {}", e))
    })?;

    // Validate schema SQL before execution to prevent malicious statements
    validate_schema_sql(&schema_content, db_type)?;

    match db_type {
        DatabaseType::PostgreSQL => {
            let pool = manager.get_pool_postgres(connection_id).await?;
            sqlx::query(&schema_content).execute(&pool).await?;
        }
        DatabaseType::MariaDB | DatabaseType::MySQL => {
            let pool = manager.get_pool_mysql(connection_id).await?;

            // Get a single connection to ensure session variables persist
            let mut conn = pool.acquire().await?;

            // Disable foreign key checks to allow dropping/creating tables in any order
            sqlx::query("SET FOREIGN_KEY_CHECKS=0")
                .execute(&mut *conn)
                .await?;

            // Split by semicolon and execute each statement
            // The schema.sql should contain DROP TABLE IF EXISTS before each CREATE TABLE
            for statement in schema_content.split(';') {
                let trimmed = statement.trim();
                if !trimmed.is_empty() && !trimmed.starts_with("--") {
                    sqlx::query(trimmed).execute(&mut *conn).await.map_err(|e| {
                        AppError::DatabaseError(format!(
                            "Failed to execute schema statement: {}. Statement: {}",
                            e,
                            if trimmed.len() > 100 {
                                &trimmed[..100]
                            } else {
                                trimmed
                            }
                        ))
                    })?;
                }
            }

            // Re-enable foreign key checks
            sqlx::query("SET FOREIGN_KEY_CHECKS=1")
                .execute(&mut *conn)
                .await?;
        }
    }

    Ok(())
}

/// Streaming CSV import - reads and processes in chunks, no full file load
async fn import_csv_to_table_streaming(
    manager: &ConnectionManager,
    connection_id: &str,
    csv_path: &PathBuf,
    table_name: &str,
    db_type: &DatabaseType,
) -> AppResult<()> {
    // Open file with buffered reader
    let file = File::open(csv_path).map_err(|e| {
        AppError::IoError(format!("Failed to open CSV file: {}", e))
    })?;

    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(BufReader::with_capacity(256 * 1024, file)); // 256KB buffer

    // Get headers
    let headers = reader
        .headers()
        .map_err(|e| AppError::IoError(format!("Failed to read CSV headers: {}", e)))?
        .clone();

    let column_names: Vec<String> = headers.iter().map(|h| h.to_string()).collect();

    if column_names.is_empty() {
        return Ok(());
    }

    // Process in batches of 1000 rows without loading entire file
    let batch_size = 1000;
    let mut batch: Vec<Vec<String>> = Vec::with_capacity(batch_size);

    for result in reader.records() {
        let record = result.map_err(|e| {
            AppError::IoError(format!("Failed to read CSV record: {}", e))
        })?;

        let values: Vec<String> = record.iter().map(|field| field.to_string()).collect();
        batch.push(values);

        // When batch is full, insert it
        if batch.len() >= batch_size {
            insert_batch(
                manager,
                connection_id,
                table_name,
                &column_names,
                &batch,
                db_type,
            )
            .await?;
            batch.clear();
        }
    }

    // Insert remaining records
    if !batch.is_empty() {
        insert_batch(
            manager,
            connection_id,
            table_name,
            &column_names,
            &batch,
            db_type,
        )
        .await?;
    }

    Ok(())
}

/// Insert a single batch
async fn insert_batch(
    manager: &ConnectionManager,
    connection_id: &str,
    table_name: &str,
    column_names: &[String],
    batch: &[Vec<String>],
    db_type: &DatabaseType,
) -> AppResult<()> {
    match db_type {
        DatabaseType::PostgreSQL => {
            insert_postgres_batch(manager, connection_id, table_name, column_names, batch).await
        }
        DatabaseType::MariaDB | DatabaseType::MySQL => {
            insert_mysql_batch(manager, connection_id, table_name, column_names, batch).await
        }
    }
}

async fn insert_postgres_batch(
    manager: &ConnectionManager,
    connection_id: &str,
    table_name: &str,
    column_names: &[String],
    batch: &[Vec<String>],
) -> AppResult<()> {
    let pool = manager.get_pool_postgres(connection_id).await?;

    // Get a dedicated connection for this batch
    let mut conn = pool.acquire().await?;

    // For PostgreSQL, defer constraints within this transaction
    // This allows inserting data regardless of FK order
    sqlx::query("SET CONSTRAINTS ALL DEFERRED")
        .execute(&mut *conn)
        .await?;

    let columns = column_names
        .iter()
        .map(|c| quote_identifier_postgres(c))
        .collect::<Vec<_>>()
        .join(", ");

    let mut placeholders = Vec::new();
    let mut values: Vec<&str> = Vec::new();
    let mut param_index = 1;

    for record in batch {
        let row_placeholders: Vec<String> = (0..column_names.len())
            .map(|_| {
                let placeholder = format!("${}", param_index);
                param_index += 1;
                placeholder
            })
            .collect();

        placeholders.push(format!("({})", row_placeholders.join(", ")));

        for value in record {
            values.push(value.as_str());
        }
    }

    let query = format!(
        "INSERT INTO {} ({}) VALUES {}",
        quote_identifier_postgres(table_name),
        columns,
        placeholders.join(", ")
    );

    let mut query_builder = sqlx::query(&query);
    for value in values {
        // Handle NULL marker from CSV export (PostgreSQL COPY convention)
        // Empty strings are now preserved as empty strings for VARCHAR/TEXT columns
        if value == CSV_NULL_MARKER {
            query_builder = query_builder.bind(None::<String>);
        } else if value.starts_with("\\x") && value.len() > 2 && value != CSV_NULL_MARKER {
            // PostgreSQL hex format for BYTEA columns
            match hex::decode(&value[2..]) {
                Ok(bytes) => query_builder = query_builder.bind(bytes),
                Err(_) => query_builder = query_builder.bind(value), // Fallback to string if not valid hex
            }
        } else if value.starts_with("0x") || value.starts_with("0X") {
            // Alternative hex format (legacy support)
            match hex::decode(&value[2..]) {
                Ok(bytes) => query_builder = query_builder.bind(bytes),
                Err(_) => query_builder = query_builder.bind(value), // Fallback to string if not valid hex
            }
        } else if value.starts_with("SRID=") || value.starts_with("POINT") ||
                  value.starts_with("LINESTRING") || value.starts_with("POLYGON") ||
                  value.starts_with("MULTIPOINT") || value.starts_with("MULTILINESTRING") ||
                  value.starts_with("MULTIPOLYGON") || value.starts_with("GEOMETRYCOLLECTION") {
            // Geospatial WKT/EWKT format - PostgreSQL will handle conversion via column type
            // The schema should have the geometry/geography column defined
            query_builder = query_builder.bind(value);
        } else if value.starts_with("{") && value.ends_with("}") {
            // PostgreSQL array format - pass as-is, PostgreSQL will parse it
            query_builder = query_builder.bind(value);
        } else {
            // Standard value - let PostgreSQL handle type conversion
            query_builder = query_builder.bind(value);
        }
    }

    query_builder.execute(&mut *conn).await?;
    Ok(())
}

async fn insert_mysql_batch(
    manager: &ConnectionManager,
    connection_id: &str,
    table_name: &str,
    column_names: &[String],
    batch: &[Vec<String>],
) -> AppResult<()> {
    let pool = manager.get_pool_mysql(connection_id).await?;

    // Get a dedicated connection for this batch
    let mut conn = pool.acquire().await?;

    // For MySQL/MariaDB, disable FK checks for this connection's session
    // This allows inserting data regardless of FK order during parallel imports
    sqlx::query("SET FOREIGN_KEY_CHECKS = 0")
        .execute(&mut *conn)
        .await?;

    let columns = column_names
        .iter()
        .map(|c| quote_identifier_mysql(c))
        .collect::<Vec<_>>()
        .join(", ");

    let placeholders: Vec<String> = batch
        .iter()
        .map(|_| {
            let row_placeholders = vec!["?"; column_names.len()];
            format!("({})", row_placeholders.join(", "))
        })
        .collect();

    let query = format!(
        "INSERT INTO {} ({}) VALUES {}",
        quote_identifier_mysql(table_name),
        columns,
        placeholders.join(", ")
    );

    let mut query_builder = sqlx::query(&query);
    for record in batch {
        for value in record {
            // Handle NULL marker from CSV export (PostgreSQL COPY convention)
            // Empty strings are now preserved as empty strings for VARCHAR/TEXT columns
            if value == CSV_NULL_MARKER {
                query_builder = query_builder.bind(None::<String>);
            } else if (value.starts_with("\\x") && value.len() > 2) ||
                      value.starts_with("0x") || value.starts_with("0X") {
                // Decode hex strings back to binary (for BLOB/VARBINARY columns)
                // Supports both PostgreSQL (\x) and MySQL (0x) hex formats
                let hex_start = if value.starts_with("\\x") { 2 } else { 2 };
                match hex::decode(&value[hex_start..]) {
                    Ok(bytes) => query_builder = query_builder.bind(bytes),
                    Err(_) => query_builder = query_builder.bind(value), // Fallback to string if not valid hex
                }
            } else {
                query_builder = query_builder.bind(value);
            }
        }
    }

    query_builder.execute(&mut *conn).await?;

    // Re-enable FK checks for this connection
    sqlx::query("SET FOREIGN_KEY_CHECKS = 1")
        .execute(&mut *conn)
        .await?;

    Ok(())
}

/// Streaming ZIP extraction - doesn't load entire files into memory
fn extract_zip_archive_streaming(zip_path: &str) -> AppResult<(Vec<PathBuf>, PathBuf)> {
    use zip::ZipArchive;

    let file = File::open(zip_path).map_err(|e| {
        AppError::IoError(format!("Failed to open ZIP file: {}", e))
    })?;

    let mut archive = ZipArchive::new(BufReader::new(file)).map_err(|e| {
        AppError::IoError(format!("Failed to read ZIP archive: {}", e))
    })?;

    let extract_dir = PathBuf::from(zip_path)
        .parent()
        .ok_or_else(|| AppError::IoError("Invalid ZIP path".to_string()))?
        .join(format!(
            ".dataspeak_import_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));

    fs::create_dir_all(&extract_dir).map_err(|e| {
        AppError::IoError(format!("Failed to create extraction directory: {}", e))
    })?;

    let mut csv_files = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| {
            AppError::IoError(format!("Failed to read ZIP entry: {}", e))
        })?;

        let file_name = file.name().to_string();

        if file_name.ends_with(".csv") || file_name.ends_with(".sql") {
            let output_path = extract_dir.join(&file_name);

            // Create parent directories if needed
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    AppError::IoError(format!("Failed to create directory: {}", e))
                })?;
            }

            let mut output_file = File::create(&output_path).map_err(|e| {
                AppError::IoError(format!("Failed to create extracted file: {}", e))
            })?;

            // Stream the file in chunks instead of loading all
            let mut buffer = [0u8; 64 * 1024]; // 64KB chunks
            loop {
                let bytes_read = file.read(&mut buffer).map_err(|e| {
                    AppError::IoError(format!("Failed to read ZIP entry: {}", e))
                })?;

                if bytes_read == 0 {
                    break;
                }

                output_file.write_all(&buffer[..bytes_read]).map_err(|e| {
                    AppError::IoError(format!("Failed to write extracted file: {}", e))
                })?;
            }

            if file_name.ends_with(".csv") {
                csv_files.push(output_path);
            }
        }
    }

    Ok((csv_files, extract_dir))
}
