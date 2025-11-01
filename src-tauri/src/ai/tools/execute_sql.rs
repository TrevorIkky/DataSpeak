use crate::ai::agent::{Tool, ToolResult};
use crate::ai::sanitizer;
use crate::db::connection::ConnectionManager;
use crate::db::query;
use crate::error::AppResult;
use std::time::Instant;

/// Execute SQL query tool for agent
pub async fn execute_sql_tool(
    tool: &Tool,
    connection_id: &str,
    connections: &ConnectionManager,
) -> AppResult<ToolResult> {
    let Tool::ExecuteSql { query } = tool;

    let start = Instant::now();

    // Sanitize the SQL query
    let sanitized_query = sanitizer::validate_sql(query)?;

    // Get connection info for additional validation
    let conn = connections.get_connection(connection_id)?;
    let db_type = match conn.database_type {
        crate::db::connection::DatabaseType::PostgreSQL => "postgres",
        crate::db::connection::DatabaseType::MySQL => "mysql",
        crate::db::connection::DatabaseType::MariaDB => "mariadb",
    };

    // Additional DB-specific validation
    sanitizer::validate_for_db_type(&sanitized_query, db_type)?;

    // Execute with existing query infrastructure
    let result = query::execute_query(
        connections,
        connection_id,
        &sanitized_query,
        100, // AI max limit
        0,   // offset
    )
    .await?;

    let execution_time = start.elapsed().as_millis();

    // Build observation message
    let observation = if result.row_count == 0 {
        "Query executed successfully but returned 0 rows.".to_string()
    } else if result.row_count == 1 {
        format!(
            "Query executed successfully. Returned 1 row in {}ms. Columns: {}",
            execution_time,
            result.columns.join(", ")
        )
    } else {
        format!(
            "Query executed successfully. Returned {} rows in {}ms. Columns: {}",
            result.row_count, execution_time,
            result.columns.join(", ")
        )
    };

    Ok(ToolResult {
        observation,
        data: Some(result),
        execution_time_ms: execution_time,
    })
}
