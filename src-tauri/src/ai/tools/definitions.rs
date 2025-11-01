use crate::ai::openrouter::types::{Tool, FunctionDefinition};

/// Build tool definitions for OpenRouter native tool calling
pub fn build_tools() -> Vec<Tool> {
    vec![Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "execute_sql".to_string(),
            description: "Execute a read-only SELECT query on the database to retrieve data. Maximum 100 rows.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The SELECT SQL query to execute. Must include a LIMIT clause (max 100). Only SELECT statements are allowed."
                    }
                },
                "required": ["query"]
            }),
        },
    }]
}
