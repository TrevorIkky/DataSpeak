use crate::db::connection::{Connection, ConnectionManager, DatabaseType};
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use futures::future::join_all;
use tauri::{AppHandle, Emitter};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// Timeout for loading individual table metadata (30 seconds)
const TABLE_QUERY_TIMEOUT: Duration = Duration::from_secs(30);

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
    pub indexes: Vec<IndexInfo>,
    pub triggers: Vec<TriggerInfo>,
    pub constraints: Vec<ConstraintInfo>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub is_primary: bool,
    pub index_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerInfo {
    pub name: String,
    pub event: String,          // INSERT, UPDATE, DELETE
    pub timing: String,          // BEFORE, AFTER
    pub statement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintInfo {
    pub name: String,
    pub constraint_type: String, // PRIMARY KEY, FOREIGN KEY, UNIQUE, CHECK
    pub columns: Vec<String>,
    pub referenced_table: Option<String>,
    pub referenced_columns: Option<Vec<String>>,
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

    // Get all tables in public schema
    let tables_query = r#"
        SELECT
            t.table_name,
            t.table_schema
        FROM information_schema.tables t
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
            let app_handle = app.clone();
            let loaded_count = Arc::clone(&loaded_count);

            async move {
                let table_name_for_error = table_name.clone();

                // Wrap all table metadata queries in a timeout
                let result = tokio::time::timeout(TABLE_QUERY_TIMEOUT, async {
                    // Get accurate row count using COUNT(*)
                    let row_count = get_postgres_row_count(&pool, &table_schema, &table_name).await?;
                    let columns = get_postgres_columns(&pool, &table_schema, &table_name).await?;
                    let indexes = get_postgres_indexes(&pool, &table_schema, &table_name).await?;
                    let triggers = get_postgres_triggers(&pool, &table_schema, &table_name).await?;
                    let constraints = get_postgres_constraints(&pool, &table_schema, &table_name).await?;

                    Ok::<Table, AppError>(Table {
                        name: table_name,
                        schema: Some(table_schema),
                        row_count,
                        columns,
                        indexes,
                        triggers,
                        constraints,
                    })
                })
                .await;

                let table = match result {
                    Ok(Ok(t)) => t,
                    Ok(Err(e)) => return Err(e),
                    Err(_) => {
                        return Err(AppError::DatabaseError(format!(
                            "Timeout loading table metadata for '{}'",
                            table_name_for_error
                        )));
                    }
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

async fn get_postgres_row_count(
    pool: &sqlx::PgPool,
    schema: &str,
    table: &str,
) -> AppResult<Option<i64>> {
    // Table and schema names come from information_schema, so they're safe
    // Escape double quotes by doubling them (PostgreSQL standard)
    let query = format!(
        "SELECT COUNT(*) FROM \"{}\".\"{}\"",
        schema.replace('"', "\"\""),
        table.replace('"', "\"\"")
    );

    let count: i64 = sqlx::query_scalar(&query)
        .fetch_one(pool)
        .await?;

    Ok(Some(count))
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

async fn get_postgres_indexes(
    pool: &sqlx::PgPool,
    schema: &str,
    table: &str,
) -> AppResult<Vec<IndexInfo>> {
    let query = r#"
        SELECT
            i.indexname as index_name,
            i.indexdef as index_definition,
            ix.indisunique as is_unique,
            ix.indisprimary as is_primary,
            am.amname as index_type,
            COALESCE(array_agg(a.attname::TEXT ORDER BY array_position(ix.indkey, a.attnum)), ARRAY[]::TEXT[]) as columns
        FROM pg_indexes i
        JOIN pg_class c ON c.relname = i.tablename
        JOIN pg_namespace n ON n.oid = c.relnamespace AND n.nspname = i.schemaname
        JOIN pg_index ix ON ix.indexrelid = (
            SELECT oid FROM pg_class WHERE relname = i.indexname AND relnamespace = n.oid
        )
        JOIN pg_class ic ON ic.oid = ix.indexrelid
        JOIN pg_am am ON am.oid = ic.relam
        JOIN pg_attribute a ON a.attrelid = c.oid AND a.attnum = ANY(ix.indkey)
        WHERE i.schemaname = $1
            AND i.tablename = $2
        GROUP BY i.indexname, i.indexdef, ix.indisunique, ix.indisprimary, am.amname
        ORDER BY i.indexname
    "#;

    let rows = sqlx::query(query)
        .bind(schema)
        .bind(table)
        .fetch_all(pool)
        .await?;

    let mut indexes = Vec::new();

    for row in rows {
        let columns_array: Vec<String> = row.try_get("columns")?;
        indexes.push(IndexInfo {
            name: row.try_get("index_name")?,
            columns: columns_array,
            is_unique: row.try_get("is_unique")?,
            is_primary: row.try_get("is_primary")?,
            index_type: row.try_get("index_type").ok(),
        });
    }

    Ok(indexes)
}

async fn get_postgres_triggers(
    pool: &sqlx::PgPool,
    schema: &str,
    table: &str,
) -> AppResult<Vec<TriggerInfo>> {
    let query = r#"
        SELECT
            t.trigger_name,
            t.event_manipulation as event,
            t.action_timing as timing,
            pg_get_triggerdef(tr.oid) as statement
        FROM information_schema.triggers t
        JOIN pg_trigger tr ON tr.tgname = t.trigger_name
        JOIN pg_class c ON c.oid = tr.tgrelid
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE t.event_object_schema = $1
            AND t.event_object_table = $2
            AND n.nspname = $1
            AND c.relname = $2
        ORDER BY t.trigger_name
    "#;

    let rows = sqlx::query(query)
        .bind(schema)
        .bind(table)
        .fetch_all(pool)
        .await?;

    let mut triggers = Vec::new();

    for row in rows {
        triggers.push(TriggerInfo {
            name: row.try_get("trigger_name")?,
            event: row.try_get("event")?,
            timing: row.try_get("timing")?,
            statement: row.try_get("statement").ok(),
        });
    }

    Ok(triggers)
}

async fn get_postgres_constraints(
    pool: &sqlx::PgPool,
    schema: &str,
    table: &str,
) -> AppResult<Vec<ConstraintInfo>> {
    let query = r#"
        SELECT
            tc.constraint_name,
            tc.constraint_type,
            COALESCE(array_agg(DISTINCT kcu.column_name::TEXT ORDER BY kcu.column_name::TEXT) FILTER (WHERE kcu.column_name IS NOT NULL), ARRAY[]::TEXT[]) as columns,
            ccu.table_name as referenced_table,
            array_agg(DISTINCT ccu.column_name::TEXT ORDER BY ccu.column_name::TEXT) FILTER (WHERE ccu.column_name IS NOT NULL) as referenced_columns
        FROM information_schema.table_constraints tc
        LEFT JOIN information_schema.key_column_usage kcu
            ON tc.constraint_name = kcu.constraint_name
            AND tc.table_schema = kcu.table_schema
        LEFT JOIN information_schema.constraint_column_usage ccu
            ON tc.constraint_name = ccu.constraint_name
            AND tc.table_schema = ccu.table_schema
        WHERE tc.table_schema = $1
            AND tc.table_name = $2
        GROUP BY tc.constraint_name, tc.constraint_type, ccu.table_name
        ORDER BY tc.constraint_name
    "#;

    let rows = sqlx::query(query)
        .bind(schema)
        .bind(table)
        .fetch_all(pool)
        .await?;

    let mut constraints = Vec::new();

    for row in rows {
        let columns_array: Vec<String> = row.try_get("columns")?;
        let referenced_columns: Option<Vec<String>> = row.try_get("referenced_columns").ok();

        constraints.push(ConstraintInfo {
            name: row.try_get("constraint_name")?,
            constraint_type: row.try_get("constraint_type")?,
            columns: columns_array,
            referenced_table: row.try_get("referenced_table").ok(),
            referenced_columns,
        });
    }

    Ok(constraints)
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
                let table_name_for_error = table_name.clone();

                // Wrap all table metadata queries in a timeout
                let result = tokio::time::timeout(TABLE_QUERY_TIMEOUT, async {
                    let columns = get_mysql_columns(&pool, &database, &table_name).await?;
                    let indexes = get_mysql_indexes(&pool, &database, &table_name).await?;
                    let triggers = get_mysql_triggers(&pool, &database, &table_name).await?;
                    let constraints = get_mysql_constraints(&pool, &database, &table_name).await?;

                    Ok::<Table, AppError>(Table {
                        name: table_name,
                        schema: None,
                        row_count,
                        columns,
                        indexes,
                        triggers,
                        constraints,
                    })
                })
                .await;

                let table = match result {
                    Ok(Ok(t)) => t,
                    Ok(Err(e)) => return Err(e),
                    Err(_) => {
                        return Err(AppError::DatabaseError(format!(
                            "Timeout loading table metadata for '{}'",
                            table_name_for_error
                        )));
                    }
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

async fn get_mysql_indexes(
    pool: &sqlx::MySqlPool,
    database: &str,
    table: &str,
) -> AppResult<Vec<IndexInfo>> {
    let query = r#"
        SELECT
            s.INDEX_NAME as index_name,
            s.NON_UNIQUE as non_unique,
            s.INDEX_TYPE as index_type,
            GROUP_CONCAT(s.COLUMN_NAME ORDER BY s.SEQ_IN_INDEX) as columns
        FROM information_schema.STATISTICS s
        WHERE s.TABLE_SCHEMA = ?
            AND s.TABLE_NAME = ?
        GROUP BY s.INDEX_NAME, s.NON_UNIQUE, s.INDEX_TYPE
        ORDER BY s.INDEX_NAME
    "#;

    let rows = sqlx::query(query)
        .bind(database)
        .bind(table)
        .fetch_all(pool)
        .await?;

    let mut indexes = Vec::new();

    for row in rows {
        let index_name: String = row.try_get("index_name")?;
        let columns_str: String = row.try_get("columns")?;
        let columns_array: Vec<String> = columns_str.split(',').map(|s| s.to_string()).collect();
        let non_unique: i32 = row.try_get("non_unique")?;

        indexes.push(IndexInfo {
            name: index_name.clone(),
            columns: columns_array,
            is_unique: non_unique == 0,
            is_primary: index_name == "PRIMARY",
            index_type: row.try_get("index_type").ok(),
        });
    }

    Ok(indexes)
}

async fn get_mysql_triggers(
    pool: &sqlx::MySqlPool,
    database: &str,
    table: &str,
) -> AppResult<Vec<TriggerInfo>> {
    let query = r#"
        SELECT
            TRIGGER_NAME as trigger_name,
            EVENT_MANIPULATION as event,
            ACTION_TIMING as timing,
            ACTION_STATEMENT as statement
        FROM information_schema.TRIGGERS
        WHERE TRIGGER_SCHEMA = ?
            AND EVENT_OBJECT_TABLE = ?
        ORDER BY TRIGGER_NAME
    "#;

    let rows = sqlx::query(query)
        .bind(database)
        .bind(table)
        .fetch_all(pool)
        .await?;

    let mut triggers = Vec::new();

    for row in rows {
        triggers.push(TriggerInfo {
            name: row.try_get("trigger_name")?,
            event: row.try_get("event")?,
            timing: row.try_get("timing")?,
            statement: row.try_get("statement").ok(),
        });
    }

    Ok(triggers)
}

async fn get_mysql_constraints(
    pool: &sqlx::MySqlPool,
    database: &str,
    table: &str,
) -> AppResult<Vec<ConstraintInfo>> {
    let query = r#"
        SELECT
            tc.CONSTRAINT_NAME as constraint_name,
            tc.CONSTRAINT_TYPE as constraint_type,
            GROUP_CONCAT(DISTINCT kcu.COLUMN_NAME ORDER BY kcu.ORDINAL_POSITION) as columns,
            kcu.REFERENCED_TABLE_NAME as referenced_table,
            GROUP_CONCAT(DISTINCT kcu.REFERENCED_COLUMN_NAME ORDER BY kcu.ORDINAL_POSITION) as referenced_columns
        FROM information_schema.TABLE_CONSTRAINTS tc
        LEFT JOIN information_schema.KEY_COLUMN_USAGE kcu
            ON tc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME
            AND tc.TABLE_SCHEMA = kcu.TABLE_SCHEMA
            AND tc.TABLE_NAME = kcu.TABLE_NAME
        WHERE tc.TABLE_SCHEMA = ?
            AND tc.TABLE_NAME = ?
        GROUP BY tc.CONSTRAINT_NAME, tc.CONSTRAINT_TYPE, kcu.REFERENCED_TABLE_NAME
        ORDER BY tc.CONSTRAINT_NAME
    "#;

    let rows = sqlx::query(query)
        .bind(database)
        .bind(table)
        .fetch_all(pool)
        .await?;

    let mut constraints = Vec::new();

    for row in rows {
        let columns_str: Option<String> = row.try_get("columns")?;
        let columns_array: Vec<String> = columns_str
            .map(|s| s.split(',').map(|x| x.to_string()).collect())
            .unwrap_or_else(Vec::new);

        let referenced_columns: Option<Vec<String>> = row
            .try_get::<Option<String>, _>("referenced_columns")
            .ok()
            .flatten()
            .map(|s| s.split(',').map(|x| x.to_string()).collect());

        constraints.push(ConstraintInfo {
            name: row.try_get("constraint_name")?,
            constraint_type: row.try_get("constraint_type")?,
            columns: columns_array,
            referenced_table: row.try_get("referenced_table").ok(),
            referenced_columns,
        });
    }

    Ok(constraints)
}
