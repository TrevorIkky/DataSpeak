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
    let Tool::ExecuteSql { query, dry_run } = tool;

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

    // If dry_run, just return the validated SQL without executing
    if *dry_run {
        let observation = format!(
            "SQL query generated and validated successfully:\n\n```sql\n{}\n```\n\nThis query is ready to be used.",
            sanitized_query
        );

        return Ok(ToolResult {
            observation,
            data: None,
        });
    }

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

    // Build observation message - just metadata, not the actual data
    // The data is sent to the UI separately, so the AI just needs to know the query succeeded
    let observation = if result.row_count == 0 {
        "Query executed successfully but returned 0 rows. The query was valid but no data matched the criteria.".to_string()
    } else {
        format!(
            "Query executed successfully in {}ms. Returned {} row{}. The results are displayed in the table above.",
            execution_time,
            result.row_count,
            if result.row_count == 1 { "" } else { "s" }
        )
    };

    Ok(ToolResult {
        observation,
        data: Some(result),
    })
}
