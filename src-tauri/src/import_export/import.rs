use crate::db::connection::{ConnectionManager, DatabaseType};
use crate::error::{AppError, AppResult};
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub file_name: String,
    pub current: usize,
    pub total: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOptions {
    pub connection_id: String,
    pub source_path: String,
    pub is_zip: bool,
    pub table_mappings: HashMap<String, String>, // CSV filename -> table name
}

pub async fn import_tables(
    app: AppHandle,
    manager: &ConnectionManager,
    options: ImportOptions,
) -> AppResult<()> {
    let conn = manager.get_connection(&options.connection_id)?;

    // Extract files if ZIP
    let csv_files = if options.is_zip {
        app.emit(
            "import-progress",
            ImportProgress {
                file_name: String::new(),
                current: 0,
                total: 1,
                status: "Extracting ZIP archive...".to_string(),
            },
        )
        .ok();

        extract_zip_archive(&options.source_path)?
    } else {
        vec![PathBuf::from(&options.source_path)]
    };

    let total_files = csv_files.len();

    // Import each CSV file
    for (idx, csv_path) in csv_files.iter().enumerate() {
        let file_name = csv_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let table_name = options
            .table_mappings
            .get(file_name)
            .cloned()
            .unwrap_or_else(|| file_name.to_string());

        app.emit(
            "import-progress",
            ImportProgress {
                file_name: file_name.to_string(),
                current: idx + 1,
                total: total_files,
                status: format!("Importing into table: {}", table_name),
            },
        )
        .ok();

        import_csv_to_table(
            manager,
            &options.connection_id,
            &csv_path,
            &table_name,
            &conn.database_type,
        )
        .await?;
    }

    // Emit completion event
    app.emit(
        "import-progress",
        ImportProgress {
            file_name: String::new(),
            current: total_files,
            total: total_files,
            status: "Import completed!".to_string(),
        },
    )
    .ok();

    Ok(())
}

async fn import_csv_to_table(
    manager: &ConnectionManager,
    connection_id: &str,
    csv_path: &PathBuf,
    table_name: &str,
    db_type: &DatabaseType,
) -> AppResult<()> {
    // Read CSV file
    let file = File::open(csv_path).map_err(|e| {
        AppError::IoError(format!("Failed to open CSV file: {}", e))
    })?;

    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(BufReader::new(file));

    // Get headers
    let headers = reader
        .headers()
        .map_err(|e| AppError::IoError(format!("Failed to read CSV headers: {}", e)))?
        .clone();

    let column_names: Vec<String> = headers.iter().map(|h| h.to_string()).collect();

    // Read all records into memory
    let records: Vec<Vec<String>> = reader
        .records()
        .filter_map(|r| r.ok())
        .map(|record| record.iter().map(|field| field.to_string()).collect())
        .collect();

    if records.is_empty() {
        return Ok(());
    }

    // Process records in parallel batches
    let batch_size = 1000;
    let batches: Vec<Vec<Vec<String>>> = records
        .chunks(batch_size)
        .map(|chunk| chunk.to_vec())
        .collect();

    match db_type {
        DatabaseType::PostgreSQL => {
            import_postgres_batches(manager, connection_id, table_name, &column_names, batches).await
        }
        DatabaseType::MariaDB | DatabaseType::MySQL => {
            import_mysql_batches(manager, connection_id, table_name, &column_names, batches).await
        }
    }
}

async fn import_postgres_batches(
    manager: &ConnectionManager,
    connection_id: &str,
    table_name: &str,
    column_names: &[String],
    batches: Vec<Vec<Vec<String>>>,
) -> AppResult<()> {
    let pool = manager.get_pool_postgres(connection_id).await?;

    for batch in batches {
        // Build INSERT query with multiple VALUES
        let columns = column_names
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ");

        let mut placeholders = Vec::new();
        let mut values: Vec<&str> = Vec::new();
        let mut param_index = 1;

        for record in &batch {
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
            "INSERT INTO \"{}\" ({}) VALUES {}",
            table_name,
            columns,
            placeholders.join(", ")
        );

        // Build query with parameters
        let mut query_builder = sqlx::query(&query);
        for value in values {
            query_builder = query_builder.bind(value);
        }

        query_builder.execute(&pool).await?;
    }

    Ok(())
}

async fn import_mysql_batches(
    manager: &ConnectionManager,
    connection_id: &str,
    table_name: &str,
    column_names: &[String],
    batches: Vec<Vec<Vec<String>>>,
) -> AppResult<()> {
    let pool = manager.get_pool_mysql(connection_id).await?;

    for batch in batches {
        // Build INSERT query with multiple VALUES
        let columns = column_names
            .iter()
            .map(|c| format!("`{}`", c))
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
            "INSERT INTO `{}` ({}) VALUES {}",
            table_name,
            columns,
            placeholders.join(", ")
        );

        // Build query with parameters
        let mut query_builder = sqlx::query(&query);
        for record in &batch {
            for value in record {
                query_builder = query_builder.bind(value);
            }
        }

        query_builder.execute(&pool).await?;
    }

    Ok(())
}

fn extract_zip_archive(zip_path: &str) -> AppResult<Vec<PathBuf>> {
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
        .join("extracted");

    std::fs::create_dir_all(&extract_dir).map_err(|e| {
        AppError::IoError(format!("Failed to create extraction directory: {}", e))
    })?;

    let mut csv_files = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| {
            AppError::IoError(format!("Failed to read ZIP entry: {}", e))
        })?;

        if file.name().ends_with(".csv") {
            let output_path = extract_dir.join(file.name());

            let mut output_file = File::create(&output_path).map_err(|e| {
                AppError::IoError(format!("Failed to create extracted file: {}", e))
            })?;

            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).map_err(|e| {
                AppError::IoError(format!("Failed to read ZIP entry contents: {}", e))
            })?;

            std::io::Write::write_all(&mut output_file, &buffer).map_err(|e| {
                AppError::IoError(format!("Failed to write extracted file: {}", e))
            })?;

            csv_files.push(output_path);
        }
    }

    Ok(csv_files)
}
