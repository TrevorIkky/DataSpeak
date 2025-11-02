use crate::ai::openrouter::types::{Tool, FunctionDefinition};

/// Build tool definitions for OpenRouter native tool calling
pub fn build_tools() -> Vec<Tool> {
    vec![Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "execute_sql".to_string(),
            description: "Execute a read-only SELECT query on the database to retrieve data. Supports all standard SQL SELECT operations including WHERE clauses, JOINs, GROUP BY, ORDER BY, and aggregate functions (COUNT, SUM, AVG, MIN, MAX). Returns up to 100 rows maximum. Use this tool to answer questions that require querying the database. The tool will return the actual data along with column names and row count.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The SELECT SQL query to execute. Must include a LIMIT clause (maximum 100 rows). Only SELECT statements are allowed - no INSERT, UPDATE, DELETE, DROP, or other modification statements. Examples: 'SELECT * FROM users LIMIT 10', 'SELECT COUNT(*) as total FROM orders WHERE status = \\'pending\\' LIMIT 1', 'SELECT category, SUM(amount) as revenue FROM sales GROUP BY category ORDER BY revenue DESC LIMIT 5'"
                    }
                },
                "required": ["query"]
            }),
        },
    }]
}
