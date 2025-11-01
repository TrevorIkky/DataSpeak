use crate::db::connection::{Connection, ConnectionManager, DatabaseType};
use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use futures::future::join_all;
use tauri::{AppHandle, Emitter};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub database_name: String,
    pub tables: Vec<Table>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaLoadProgress {
    pub table: Table,
    pub loaded: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub schema: Option<String>,
    pub row_count: Option<i64>,
    pub columns: Vec<ColumnInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
    pub foreign_key_table: Option<String>,
    pub foreign_key_column: Option<String>,
    pub default_value: Option<String>,
    pub character_maximum_length: Option<i32>,
}

pub async fn get_schema(
    manager: &ConnectionManager,
    connection_id: &str,
    app: &AppHandle,
) -> AppResult<Schema> {
    let conn = manager.get_connection(connection_id)?;

    match conn.database_type {
        DatabaseType::PostgreSQL => get_postgres_schema(manager, connection_id, &conn, app).await,
        DatabaseType::MariaDB | DatabaseType::MySQL => {
            get_mysql_schema(manager, connection_id, &conn, app).await
        }
    }
}

async fn get_postgres_schema(
    manager: &ConnectionManager,
    connection_id: &str,
    conn: &Connection,
    app: &AppHandle,
) -> AppResult<Schema> {
    let pool = manager.get_pool_postgres(connection_id).await?;

    // Get all tables in public schema with approximate row counts
    // Using pg_class.reltuples for fast approximate counts instead of COUNT(*)
    let tables_query = r#"
        SELECT
            t.table_name,
            t.table_schema,
            c.reltuples::bigint as row_count
        FROM information_schema.tables t
        LEFT JOIN pg_class c ON c.relname = t.table_name
        LEFT JOIN pg_namespace n ON n.oid = c.relnamespace AND n.nspname = t.table_schema
        WHERE t.table_schema = 'public'
        AND t.table_type = 'BASE TABLE'
        ORDER BY t.table_name
    "#;

    let table_rows = sqlx::query(tables_query).fetch_all(&pool).await?;
    let total_tables = table_rows.len();
    let loaded_count = Arc::new(AtomicUsize::new(0));

    // Create futures for loading columns for all tables in parallel
    let column_futures: Vec<_> = table_rows
        .iter()
        .map(|table_row| {
            let pool = pool.clone();
            let table_name: String = table_row.try_get("table_name").unwrap();
            let table_schema: String = table_row.try_get("table_schema").unwrap();
            let row_count: Option<i64> = table_row.try_get("row_count").ok();
            let app_handle = app.clone();
            let loaded_count = Arc::clone(&loaded_count);

            async move {
                let columns = get_postgres_columns(&pool, &table_schema, &table_name).await?;
                let table = Table {
                    name: table_name,
                    schema: Some(table_schema),
                    row_count,
                    columns,
                };

                // Increment counter and emit event
                let loaded = loaded_count.fetch_add(1, Ordering::SeqCst) + 1;
                let progress = SchemaLoadProgress {
                    table: table.clone(),
                    loaded,
                    total: total_tables,
                };

                let _ = app_handle.emit("schema-load-progress", progress);

                Ok::<Table, crate::error::AppError>(table)
            }
        })
        .collect();

    // Execute all column queries concurrently
    let results = join_all(column_futures).await;

    // Collect results and handle errors
    let mut tables = Vec::new();
    for result in results {
        tables.push(result?);
    }

    Ok(Schema {
        database_name: conn.default_database.clone(),
        tables,
    })
}

async fn get_postgres_columns(
    pool: &sqlx::PgPool,
    schema: &str,
    table: &str,
) -> AppResult<Vec<ColumnInfo>> {
    let query = r#"
        SELECT
            c.column_name,
            c.data_type,
            c.is_nullable,
            c.column_default,
            c.character_maximum_length,
            CASE WHEN pk.column_name IS NOT NULL THEN true ELSE false END as is_primary_key,
            CASE WHEN fk.column_name IS NOT NULL THEN true ELSE false END as is_foreign_key,
            fk.foreign_table_name,
            fk.foreign_column_name
        FROM information_schema.columns c
        LEFT JOIN (
            SELECT ku.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage ku
                ON tc.constraint_name = ku.constraint_name
            WHERE tc.constraint_type = 'PRIMARY KEY'
                AND tc.table_schema = $1
                AND tc.table_name = $2
        ) pk ON c.column_name = pk.column_name
        LEFT JOIN (
            SELECT
                kcu.column_name,
                ccu.table_name AS foreign_table_name,
                ccu.column_name AS foreign_column_name
            FROM information_schema.table_constraints AS tc
            JOIN information_schema.key_column_usage AS kcu
                ON tc.constraint_name = kcu.constraint_name
            JOIN information_schema.constraint_column_usage AS ccu
                ON ccu.constraint_name = tc.constraint_name
            WHERE tc.constraint_type = 'FOREIGN KEY'
                AND tc.table_schema = $1
                AND tc.table_name = $2
        ) fk ON c.column_name = fk.column_name
        WHERE c.table_schema = $1
            AND c.table_name = $2
        ORDER BY c.ordinal_position
    "#;

    let rows = sqlx::query(query)
        .bind(schema)
        .bind(table)
        .fetch_all(pool)
        .await?;

    let mut columns = Vec::new();

    for row in rows {
        columns.push(ColumnInfo {
            name: row.try_get("column_name")?,
            data_type: row.try_get("data_type")?,
            is_nullable: row.try_get::<String, _>("is_nullable")? == "YES",
            is_primary_key: row.try_get("is_primary_key")?,
            is_foreign_key: row.try_get("is_foreign_key")?,
            foreign_key_table: row.try_get("foreign_table_name").ok(),
            foreign_key_column: row.try_get("foreign_column_name").ok(),
            default_value: row.try_get("column_default").ok(),
            character_maximum_length: row.try_get("character_maximum_length").ok(),
        });
    }

    Ok(columns)
}

async fn get_mysql_schema(
    manager: &ConnectionManager,
    connection_id: &str,
    conn: &Connection,
    app: &AppHandle,
) -> AppResult<Schema> {
    let pool = manager.get_pool_mysql(connection_id).await?;

    // Get all tables with approximate row counts from information_schema
    // TABLE_ROWS is an estimate but much faster than COUNT(*)
    let tables_query = "SELECT table_name, table_rows FROM information_schema.tables WHERE table_schema = ? AND table_type = 'BASE TABLE' ORDER BY table_name";

    let table_rows = sqlx::query(tables_query)
        .bind(&conn.default_database)
        .fetch_all(&pool)
        .await?;

    let total_tables = table_rows.len();
    let loaded_count = Arc::new(AtomicUsize::new(0));

    // Create futures for loading columns for all tables in parallel
    let column_futures: Vec<_> = table_rows
        .iter()
        .map(|table_row| {
            let pool = pool.clone();
            let database = conn.default_database.clone();
            let table_name: String = table_row.try_get("table_name").unwrap();
            let row_count: Option<i64> = table_row.try_get::<Option<u64>, _>("table_rows").ok().flatten().map(|v| v as i64);
            let app_handle = app.clone();
            let loaded_count = Arc::clone(&loaded_count);

            async move {
                let columns = get_mysql_columns(&pool, &database, &table_name).await?;
                let table = Table {
                    name: table_name,
                    schema: None,
                    row_count,
                    columns,
                };

                // Increment counter and emit event
                let loaded = loaded_count.fetch_add(1, Ordering::SeqCst) + 1;
                let progress = SchemaLoadProgress {
                    table: table.clone(),
                    loaded,
                    total: total_tables,
                };

                let _ = app_handle.emit("schema-load-progress", progress);

                Ok::<Table, crate::error::AppError>(table)
            }
        })
        .collect();

    // Execute all column queries concurrently
    let results = join_all(column_futures).await;

    // Collect results and handle errors
    let mut tables = Vec::new();
    for result in results {
        tables.push(result?);
    }

    Ok(Schema {
        database_name: conn.default_database.clone(),
        tables,
    })
}

async fn get_mysql_columns(
    pool: &sqlx::MySqlPool,
    database: &str,
    table: &str,
) -> AppResult<Vec<ColumnInfo>> {
    let query = r#"
        SELECT
            c.COLUMN_NAME as column_name,
            c.DATA_TYPE as data_type,
            c.IS_NULLABLE as is_nullable,
            c.COLUMN_DEFAULT as column_default,
            c.CHARACTER_MAXIMUM_LENGTH as character_maximum_length,
            c.COLUMN_KEY as column_key,
            k.REFERENCED_TABLE_NAME as foreign_table_name,
            k.REFERENCED_COLUMN_NAME as foreign_column_name
        FROM information_schema.COLUMNS c
        LEFT JOIN (
            SELECT
                TABLE_SCHEMA,
                TABLE_NAME,
                COLUMN_NAME,
                REFERENCED_TABLE_NAME,
                REFERENCED_COLUMN_NAME
            FROM information_schema.KEY_COLUMN_USAGE
            WHERE REFERENCED_TABLE_NAME IS NOT NULL
                AND TABLE_SCHEMA = ?
                AND TABLE_NAME = ?
        ) k ON c.TABLE_SCHEMA = k.TABLE_SCHEMA
            AND c.TABLE_NAME = k.TABLE_NAME
            AND c.COLUMN_NAME = k.COLUMN_NAME
        WHERE c.TABLE_SCHEMA = ?
            AND c.TABLE_NAME = ?
        ORDER BY c.ORDINAL_POSITION
        "#;

    let rows = sqlx::query(query)
        .bind(database)
        .bind(table)
        .bind(database)
        .bind(table)
        .fetch_all(pool)
        .await?;

    let mut columns = Vec::new();

    for row in rows {
        let column_key: String = row.try_get("column_key").unwrap_or_default();

        columns.push(ColumnInfo {
            name: row.try_get("column_name")?,
            data_type: row.try_get("data_type")?,
            is_nullable: row.try_get::<String, _>("is_nullable")? == "YES",
            is_primary_key: column_key == "PRI",
            is_foreign_key: row.try_get::<Option<String>, _>("foreign_table_name")?.is_some(),
            foreign_key_table: row.try_get("foreign_table_name").ok(),
            foreign_key_column: row.try_get("foreign_column_name").ok(),
            default_value: row.try_get("column_default").ok(),
            character_maximum_length: row.try_get::<Option<u64>, _>("character_maximum_length")?.map(|v| v as i32),
        });
    }

    Ok(columns)
}
