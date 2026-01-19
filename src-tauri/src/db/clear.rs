use crate::db::connection::{ConnectionManager, DatabaseType};
use crate::error::AppResult;
use futures::future::join_all;

/// Safely quote a PostgreSQL identifier (table name)
fn quote_identifier_postgres(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

/// Safely quote a MySQL identifier (table name)
fn quote_identifier_mysql(identifier: &str) -> String {
    format!("`{}`", identifier.replace('`', "``"))
}

/// Clear all data from tables (TRUNCATE - keeps table structures)
pub async fn clear_data_only(
    manager: &ConnectionManager,
    connection_id: &str,
) -> AppResult<()> {
    let conn = manager.get_connection(connection_id)?;

    match conn.database_type {
        DatabaseType::PostgreSQL => truncate_postgres_tables(manager, connection_id).await,
        DatabaseType::MariaDB | DatabaseType::MySQL => truncate_mysql_tables(manager, connection_id).await,
    }
}

/// Clear entire database (DROP - removes all tables)
pub async fn clear_database(
    manager: &ConnectionManager,
    connection_id: &str,
) -> AppResult<()> {
    let conn = manager.get_connection(connection_id)?;

    match conn.database_type {
        DatabaseType::PostgreSQL => drop_postgres_tables(manager, connection_id).await,
        DatabaseType::MariaDB | DatabaseType::MySQL => drop_mysql_tables(manager, connection_id).await,
    }
}

// PostgreSQL - TRUNCATE (clear data only)
async fn truncate_postgres_tables(
    manager: &ConnectionManager,
    connection_id: &str,
) -> AppResult<()> {
    let pool = manager.get_pool_postgres(connection_id).await?;

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT tablename FROM pg_tables WHERE schemaname = 'public'"
    )
    .fetch_all(&pool)
    .await?;

    if tables.is_empty() {
        return Ok(());
    }

    let quoted_tables: Vec<String> = tables
        .iter()
        .map(|t| quote_identifier_postgres(t))
        .collect();

    let query = format!(
        "TRUNCATE TABLE {} RESTART IDENTITY CASCADE",
        quoted_tables.join(", ")
    );

    sqlx::query(&query).execute(&pool).await?;

    Ok(())
}

// PostgreSQL - DROP (remove tables)
async fn drop_postgres_tables(
    manager: &ConnectionManager,
    connection_id: &str,
) -> AppResult<()> {
    let pool = manager.get_pool_postgres(connection_id).await?;

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT tablename FROM pg_tables WHERE schemaname = 'public'"
    )
    .fetch_all(&pool)
    .await?;

    if tables.is_empty() {
        return Ok(());
    }

    let quoted_tables: Vec<String> = tables
        .iter()
        .map(|t| quote_identifier_postgres(t))
        .collect();

    let query = format!(
        "DROP TABLE IF EXISTS {} CASCADE",
        quoted_tables.join(", ")
    );

    sqlx::query(&query).execute(&pool).await?;

    Ok(())
}

// MySQL/MariaDB - TRUNCATE (clear data only)
async fn truncate_mysql_tables(
    manager: &ConnectionManager,
    connection_id: &str,
) -> AppResult<()> {
    let pool = manager.get_pool_mysql(connection_id).await?;
    let conn_info = manager.get_connection(connection_id)?;

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = ?"
    )
    .bind(&conn_info.default_database)
    .fetch_all(&pool)
    .await?;

    if tables.is_empty() {
        return Ok(());
    }

    // Strategy: Each connection disables FK checks for its session, then parallel truncate
    // This ensures FK checks are disabled for each connection that does the work

    let chunk_size = 10; // Larger chunks for better throughput
    let mut futures = Vec::new();

    for chunk in tables.chunks(chunk_size) {
        let pool_clone = pool.clone();
        let tables_chunk: Vec<String> = chunk.to_vec();

        let future = async move {
            let mut conn = pool_clone.acquire().await?;

            // Disable FK checks for this connection's session
            sqlx::query("SET FOREIGN_KEY_CHECKS = 0")
                .execute(&mut *conn)
                .await?;

            for table in &tables_chunk {
                let quoted_table = quote_identifier_mysql(table);
                sqlx::query(&format!("TRUNCATE TABLE {}", quoted_table))
                    .execute(&mut *conn)
                    .await?;
            }

            // Re-enable FK checks for this connection's session
            sqlx::query("SET FOREIGN_KEY_CHECKS = 1")
                .execute(&mut *conn)
                .await?;

            Ok::<_, sqlx::Error>(())
        };

        futures.push(future);
    }

    let results = join_all(futures).await;

    // Check for errors
    for result in results {
        result?;
    }

    Ok(())
}

// MySQL/MariaDB - DROP (remove tables)
async fn drop_mysql_tables(
    manager: &ConnectionManager,
    connection_id: &str,
) -> AppResult<()> {
    let pool = manager.get_pool_mysql(connection_id).await?;
    let conn_info = manager.get_connection(connection_id)?;

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = ?"
    )
    .bind(&conn_info.default_database)
    .fetch_all(&pool)
    .await?;

    if tables.is_empty() {
        return Ok(());
    }

    // Strategy: Use single connection with batched DROP statements for speed
    // Batching avoids metadata lock contention while keeping it fast
    let mut conn = pool.acquire().await?;

    // Disable FK checks for this connection's session
    sqlx::query("SET FOREIGN_KEY_CHECKS = 0")
        .execute(&mut *conn)
        .await?;

    // Drop tables in batches of 10 for optimal speed/reliability balance
    for chunk in tables.chunks(10) {
        let quoted_tables: Vec<String> = chunk
            .iter()
            .map(|t| quote_identifier_mysql(t))
            .collect();

        let query = format!("DROP TABLE IF EXISTS {}", quoted_tables.join(", "));
        sqlx::query(&query).execute(&mut *conn).await?;
    }

    // Re-enable FK checks for this connection's session
    sqlx::query("SET FOREIGN_KEY_CHECKS = 1")
        .execute(&mut *conn)
        .await?;

    Ok(())
}
