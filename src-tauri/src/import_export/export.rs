use crate::db::connection::{ConnectionManager, DatabaseType};
use crate::error::{AppError, AppResult};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::{Column, Row};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportProgress {
    pub table_name: String,
    pub current: usize,
    pub total: usize,
    pub status: String,
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
    // Create output directory
    let output_path = PathBuf::from(&options.output_dir);
    fs::create_dir_all(&output_path).map_err(|e| {
        AppError::IoError(format!("Failed to create output directory: {}", e))
    })?;

    let conn = manager.get_connection(&options.connection_id)?;
    let total_tables = options.tables.len();

    // Emit start event
    app.emit(
        "export-progress",
        ExportProgress {
            table_name: String::new(),
            current: 0,
            total: total_tables,
            status: "Starting export...".to_string(),
        },
    )
    .ok();

    // Export tables sequentially (async operations)
    for (idx, table_name) in options.tables.iter().enumerate() {
        app.emit(
            "export-progress",
            ExportProgress {
                table_name: table_name.clone(),
                current: idx + 1,
                total: total_tables,
                status: format!("Exporting table: {}", table_name),
            },
        )
        .ok();

        export_table_to_csv(manager, &options.connection_id, table_name, &output_path, &conn.database_type).await?;
    }

    // Create ZIP if requested
    let result_path = if options.create_zip {
        app.emit(
            "export-progress",
            ExportProgress {
                table_name: String::new(),
                current: total_tables,
                total: total_tables,
                status: "Creating ZIP archive...".to_string(),
            },
        )
        .ok();

        let zip_path = create_zip_archive(&output_path)?;

        // Clean up CSV files
        for table in &options.tables {
            let csv_path = output_path.join(format!("{}.csv", table));
            fs::remove_file(&csv_path).ok();
        }

        zip_path
    } else {
        output_path.to_string_lossy().to_string()
    };

    // Emit completion event
    app.emit(
        "export-progress",
        ExportProgress {
            table_name: String::new(),
            current: total_tables,
            total: total_tables,
            status: "Export completed!".to_string(),
        },
    )
    .ok();

    Ok(result_path)
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

    // Get all rows
    let query = format!("SELECT * FROM \"{}\"", table_name);
    let rows = sqlx::query(&query).fetch_all(&pool).await?;

    if rows.is_empty() {
        // Create empty CSV with headers only
        let csv_path = output_path.join(format!("{}.csv", table_name));
        let mut file = File::create(&csv_path).map_err(|e| {
            AppError::IoError(format!("Failed to create CSV file: {}", e))
        })?;
        writeln!(file, "").ok();
        return Ok(());
    }

    // Get column names
    let columns: Vec<String> = rows[0]
        .columns()
        .iter()
        .map(|col| col.name().to_string())
        .collect();

    // Convert rows to CSV format using rayon for parallel processing
    let csv_rows: Vec<String> = rows
        .par_iter()
        .map(|row| {
            let values: Vec<String> = row
                .columns()
                .iter()
                .enumerate()
                .map(|(idx, _)| {
                    // Try to get the value - handle different types
                    if let Ok(val) = row.try_get::<Option<String>, _>(idx) {
                        escape_csv_value(&val.unwrap_or_default())
                    } else if let Ok(val) = row.try_get::<Option<i32>, _>(idx) {
                        val.map(|v| v.to_string()).unwrap_or_default()
                    } else if let Ok(val) = row.try_get::<Option<i64>, _>(idx) {
                        val.map(|v| v.to_string()).unwrap_or_default()
                    } else if let Ok(val) = row.try_get::<Option<f64>, _>(idx) {
                        val.map(|v| v.to_string()).unwrap_or_default()
                    } else if let Ok(val) = row.try_get::<Option<bool>, _>(idx) {
                        val.map(|v| v.to_string()).unwrap_or_default()
                    } else {
                        String::new()
                    }
                })
                .collect();
            values.join(",")
        })
        .collect();

    // Write to file
    let csv_path = output_path.join(format!("{}.csv", table_name));
    let mut file = File::create(&csv_path).map_err(|e| {
        AppError::IoError(format!("Failed to create CSV file: {}", e))
    })?;

    // Write header
    writeln!(file, "{}", columns.join(",")).map_err(|e| {
        AppError::IoError(format!("Failed to write CSV header: {}", e))
    })?;

    // Write data rows
    for row in csv_rows {
        writeln!(file, "{}", row).map_err(|e| {
            AppError::IoError(format!("Failed to write CSV row: {}", e))
        })?;
    }

    Ok(())
}

async fn export_mysql_table(
    manager: &ConnectionManager,
    connection_id: &str,
    table_name: &str,
    output_path: &PathBuf,
) -> AppResult<()> {
    let pool = manager.get_pool_mysql(connection_id).await?;

    // Get all rows
    let query = format!("SELECT * FROM `{}`", table_name);
    let rows = sqlx::query(&query).fetch_all(&pool).await?;

    if rows.is_empty() {
        // Create empty CSV with headers only
        let csv_path = output_path.join(format!("{}.csv", table_name));
        let mut file = File::create(&csv_path).map_err(|e| {
            AppError::IoError(format!("Failed to create CSV file: {}", e))
        })?;
        writeln!(file, "").ok();
        return Ok(());
    }

    // Get column names
    let columns: Vec<String> = rows[0]
        .columns()
        .iter()
        .map(|col| col.name().to_string())
        .collect();

    // Convert rows to CSV format using rayon for parallel processing
    let csv_rows: Vec<String> = rows
        .par_iter()
        .map(|row| {
            let values: Vec<String> = row
                .columns()
                .iter()
                .enumerate()
                .map(|(idx, _)| {
                    // Try to get the value - handle different types
                    if let Ok(val) = row.try_get::<Option<String>, _>(idx) {
                        escape_csv_value(&val.unwrap_or_default())
                    } else if let Ok(val) = row.try_get::<Option<i32>, _>(idx) {
                        val.map(|v| v.to_string()).unwrap_or_default()
                    } else if let Ok(val) = row.try_get::<Option<i64>, _>(idx) {
                        val.map(|v| v.to_string()).unwrap_or_default()
                    } else if let Ok(val) = row.try_get::<Option<f64>, _>(idx) {
                        val.map(|v| v.to_string()).unwrap_or_default()
                    } else if let Ok(val) = row.try_get::<Option<bool>, _>(idx) {
                        val.map(|v| v.to_string()).unwrap_or_default()
                    } else {
                        String::new()
                    }
                })
                .collect();
            values.join(",")
        })
        .collect();

    // Write to file
    let csv_path = output_path.join(format!("{}.csv", table_name));
    let mut file = File::create(&csv_path).map_err(|e| {
        AppError::IoError(format!("Failed to create CSV file: {}", e))
    })?;

    // Write header
    writeln!(file, "{}", columns.join(",")).map_err(|e| {
        AppError::IoError(format!("Failed to write CSV header: {}", e))
    })?;

    // Write data rows
    for row in csv_rows {
        writeln!(file, "{}", row).map_err(|e| {
            AppError::IoError(format!("Failed to write CSV row: {}", e))
        })?;
    }

    Ok(())
}

fn escape_csv_value(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn create_zip_archive(source_dir: &PathBuf) -> AppResult<String> {
    use std::io::Read;
    use zip::write::FileOptions;
    use zip::CompressionMethod;

    let zip_path = source_dir.with_extension("zip");
    let file = File::create(&zip_path).map_err(|e| {
        AppError::IoError(format!("Failed to create ZIP file: {}", e))
    })?;

    let mut zip = zip::ZipWriter::new(file);
    let options: FileOptions<()> = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o755);

    // Add all CSV files in the directory
    let entries = fs::read_dir(source_dir).map_err(|e| {
        AppError::IoError(format!("Failed to read directory: {}", e))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            AppError::IoError(format!("Failed to read directory entry: {}", e))
        })?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("csv") {
            let file_name = path
                .file_name()
                .ok_or_else(|| AppError::IoError("Invalid file name".to_string()))?
                .to_string_lossy()
                .to_string();

            zip.start_file(&file_name, options).map_err(|e| {
                AppError::IoError(format!("Failed to start ZIP file entry: {}", e))
            })?;

            let mut f = File::open(&path).map_err(|e| {
                AppError::IoError(format!("Failed to open file: {}", e))
            })?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer).map_err(|e| {
                AppError::IoError(format!("Failed to read file: {}", e))
            })?;

            zip.write_all(&buffer).map_err(|e| {
                AppError::IoError(format!("Failed to write to ZIP: {}", e))
            })?;
        }
    }

    zip.finish().map_err(|e| {
        AppError::IoError(format!("Failed to finalize ZIP: {}", e))
    })?;

    Ok(zip_path.to_string_lossy().to_string())
}
