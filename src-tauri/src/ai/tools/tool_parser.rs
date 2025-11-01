use crate::ai::agent::{Tool, ToolCall};
use crate::error::{AppError, AppResult};
use regex::Regex;
use std::sync::LazyLock;

/// Regex to extract Action and Action Input from LLM response
static ACTION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)Action:\s*(\w+)\s*Action Input:\s*(.+)").unwrap()
});

/// Parse tool call from LLM response following ReAct format
pub fn parse(response: &str) -> AppResult<ToolCall> {
    // Try to extract Action and Action Input
    if let Some(captures) = ACTION_PATTERN.captures(response) {
        let action = captures
            .get(1)
            .map(|m| m.as_str().trim())
            .ok_or_else(|| AppError::AgentError("No action found".into()))?;

        let mut action_input = captures
            .get(2)
            .map(|m| m.as_str())
            .ok_or_else(|| AppError::AgentError("No action input found".into()))?;

        // Manually trim at stopping markers (since Rust regex doesn't support lookahead)
        for marker in &["\nObservation:", "\nThought:", "\nFinal Answer:"] {
            if let Some(idx) = action_input.find(marker) {
                action_input = &action_input[..idx];
            }
        }
        let action_input = action_input.trim();

        // Parse based on action type
        let tool = match action.to_lowercase().as_str() {
            "execute_sql" => {
                // Extract SQL query (remove quotes if present)
                let query = action_input.trim_matches('"').trim_matches('\'').trim();
                Tool::ExecuteSql {
                    query: query.to_string(),
                }
            }
            _ => {
                return Err(AppError::AgentError(format!(
                    "Unknown action: {}. Only 'execute_sql' is supported. Use: Action: execute_sql",
                    action
                )))
            }
        };

        Ok(ToolCall {
            tool,
            raw_response: response.to_string(),
        })
    } else {
        Err(AppError::AgentError(
            "Could not parse action from response. Expected format:\nAction: tool_name\nAction Input: input_data".into(),
        ))
    }
}

/// Extract final answer from LLM response
pub fn extract_final_answer(response: &str) -> AppResult<String> {
    // Look for "Final Answer:" marker
    if let Some(idx) = response.find("Final Answer:") {
        let answer = response[idx + "Final Answer:".len()..].trim();
        if answer.is_empty() {
            return Err(AppError::AgentError("Final answer is empty".into()));
        }
        Ok(answer.to_string())
    } else {
        Err(AppError::AgentError(
            "No 'Final Answer:' marker found in response".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_execute_sql() {
        let response = r#"
Thought: I need to get all users
Action: execute_sql
Action Input: SELECT * FROM users LIMIT 10
        "#;

        let result = parse(response);
        assert!(result.is_ok());

        let tool_call = result.unwrap();
        match tool_call.tool {
            Tool::ExecuteSql { query } => {
                assert_eq!(query, "SELECT * FROM users LIMIT 10");
            }
            _ => panic!("Wrong tool type"),
        }
    }

    #[test]
    fn test_parse_table_query() {
        let response = r#"
Thought: Let me get all products
Action: execute_sql
Action Input: SELECT * FROM products LIMIT 5
        "#;

        let result = parse(response);
        assert!(result.is_ok());

        let tool_call = result.unwrap();
        match tool_call.tool {
            Tool::ExecuteSql { query } => {
                assert_eq!(query, "SELECT * FROM products LIMIT 5");
            }
        }
    }

    #[test]
    fn test_extract_final_answer() {
        let response = r#"
Thought: I have all the information I need
Final Answer: There are 42 users in the database.
        "#;

        let result = extract_final_answer(response);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "There are 42 users in the database.");
    }

    #[test]
    fn test_parse_multiline_query() {
        let response = r#"
Thought: I'll use a JOIN query
Action: execute_sql
Action Input: SELECT u.name, COUNT(o.id) as order_count
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
GROUP BY u.id
        "#;

        let result = parse(response);
        assert!(result.is_ok());
    }
}
