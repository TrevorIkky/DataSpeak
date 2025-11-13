use crate::ai::openrouter::types::{Tool, FunctionDefinition};

/// Build tool definitions for OpenRouter native tool calling
pub fn build_tools() -> Vec<Tool> {
    vec![Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "execute_sql".to_string(),
            description: "Execute a read-only SELECT query on the database to retrieve data, or generate a SQL query without executing it. Supports all standard SQL SELECT operations including WHERE clauses, JOINs, GROUP BY, ORDER BY, and aggregate functions (COUNT, SUM, AVG, MIN, MAX). Returns up to 100 rows maximum when executed. Use this tool to answer questions that require querying the database. The tool will return the actual data along with column names and row count, or just the SQL query if dry_run is true.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The SELECT SQL query to execute or generate. Must include a LIMIT clause (maximum 100 rows). Only SELECT statements are allowed - no INSERT, UPDATE, DELETE, DROP, or other modification statements. Examples: 'SELECT * FROM users LIMIT 10', 'SELECT COUNT(*) as total FROM orders WHERE status = \\'pending\\' LIMIT 1', 'SELECT category, SUM(amount) as revenue FROM sales GROUP BY category ORDER BY revenue DESC LIMIT 5'"
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "If true, returns the SQL query without executing it. Use this when the user wants to generate SQL code rather than get the query results. Default is false (execute the query).",
                        "default": false
                    }
                },
                "required": ["query"]
            }),
        },
    }]
}
