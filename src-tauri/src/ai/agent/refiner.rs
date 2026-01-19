use crate::ai::openrouter::OpenRouterClient;
use crate::ai::agent::Message;
use crate::ai::sanitizer;
use crate::db::connection::ConnectionManager;
use crate::db::query::{self, QueryResult};
use crate::db::schema::Schema;
use crate::error::{AppError, AppResult};

/// Result from a single query refinement attempt
#[derive(Debug, Clone)]
pub struct RefinementAttempt {
    /// The SQL query that was attempted
    pub sql: String,
    /// Whether execution succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Final result from the Refiner Agent
#[derive(Debug, Clone)]
pub struct RefinerResult {
    /// The final, validated SQL query
    pub final_sql: String,
    /// The query result
    pub result: QueryResult,
    /// Number of refinement attempts
    pub attempts: u32,
}

/// Refiner Agent: Validates and corrects SQL queries
///
/// This is the third stage of the MAC-SQL pipeline. It:
/// 1. Validates and sanitizes the SQL
/// 2. Executes the query
/// 3. On failure: uses LLM to generate a corrected query
/// 4. Iterates until success or max attempts reached
pub struct RefinerAgent<'a> {
    client: &'a OpenRouterClient,
    model: &'a str,
    max_attempts: u32,
}

impl<'a> RefinerAgent<'a> {
    pub fn new(client: &'a OpenRouterClient, model: &'a str) -> Self {
        Self {
            client,
            model,
            max_attempts: 3,
        }
    }

    /// Refine and execute a SQL query with self-correction
    pub async fn refine_and_execute(
        &self,
        original_sql: &str,
        original_question: &str,
        schema: &Schema,
        db_type: &str,
        connection_id: &str,
        connections: &ConnectionManager,
    ) -> AppResult<RefinerResult> {
        let mut current_sql = original_sql.to_string();
        let mut history: Vec<RefinementAttempt> = Vec::new();
        let mut attempts = 0;

        while attempts < self.max_attempts {
            attempts += 1;

            // Try to execute the current SQL
            match self.try_execute(&current_sql, db_type, connection_id, connections).await {
                Ok(result) => {
                    // Success!
                    return Ok(RefinerResult {
                        final_sql: current_sql,
                        result,
                        attempts,
                    });
                }
                Err(error) => {
                    // Record the failed attempt
                    history.push(RefinementAttempt {
                        sql: current_sql.clone(),
                        success: false,
                        error: Some(error.to_string()),
                    });

                    // If we've hit max attempts, return the error
                    if attempts >= self.max_attempts {
                        return Err(AppError::AgentError(format!(
                            "Query refinement failed after {} attempts. Last error: {}",
                            attempts, error
                        )));
                    }

                    // Try to refine the query
                    current_sql = self.generate_corrected_sql(
                        original_question,
                        &current_sql,
                        &error.to_string(),
                        schema,
                        db_type,
                        &history,
                    ).await?;
                }
            }
        }

        Err(AppError::AgentError(format!(
            "Query refinement exhausted {} attempts",
            self.max_attempts
        )))
    }

    /// Try to execute a SQL query, returning the result or error
    async fn try_execute(
        &self,
        sql: &str,
        db_type: &str,
        connection_id: &str,
        connections: &ConnectionManager,
    ) -> AppResult<QueryResult> {
        // First, sanitize the SQL
        let sanitized = sanitizer::validate_sql(sql)?;

        // Validate for the specific database type
        sanitizer::validate_for_db_type(&sanitized, db_type)?;

        // Execute the query
        query::execute_query(
            connections,
            connection_id,
            &sanitized,
            100, // Max rows
            0,   // Offset
        ).await
    }

    /// Generate a corrected SQL query using the LLM
    async fn generate_corrected_sql(
        &self,
        original_question: &str,
        failed_sql: &str,
        error_message: &str,
        schema: &Schema,
        db_type: &str,
        history: &[RefinementAttempt],
    ) -> AppResult<String> {
        // Build context from previous attempts
        let attempt_history = if history.len() > 1 {
            let prev_attempts: Vec<String> = history.iter()
                .filter(|a| !a.success)
                .map(|a| format!(
                    "Attempt:\n```sql\n{}\n```\nError: {}",
                    a.sql,
                    a.error.as_ref().unwrap_or(&"Unknown error".to_string())
                ))
                .collect();
            format!("\n\nPrevious failed attempts:\n{}", prev_attempts.join("\n\n"))
        } else {
            String::new()
        };

        let schema_str = self.format_schema_for_error(schema, error_message);

        let system_prompt = format!(
            r#"You are a SQL error correction expert. A SQL query failed to execute and you need to fix it.

DATABASE TYPE: {} (use {}-compatible syntax)

RELEVANT SCHEMA:
{}

ORIGINAL QUESTION: {}

FAILED SQL:
```sql
{}
```

ERROR:
{}
{}

INSTRUCTIONS:
1. Analyze the error message carefully
2. Check the schema for correct table/column names
3. Verify SQL syntax for {} database
4. Generate a CORRECTED SQL query

COMMON FIXES:
- Table not found: Check schema for exact table name (case-sensitive in some databases)
- Column not found: Verify column exists in the table
- Syntax error: Check for missing quotes, commas, or parentheses
- Type mismatch: Ensure comparisons use matching types
- Missing LIMIT: Always include LIMIT clause (max 100)

Respond with ONLY the corrected SQL query, no explanation. The query must:
- Be a valid SELECT statement
- Include LIMIT clause (max 100)
- Use correct {} syntax"#,
            db_type, db_type,
            schema_str,
            original_question,
            failed_sql,
            error_message,
            attempt_history,
            db_type, db_type
        );

        let messages = vec![
            Message::system(system_prompt),
            Message::user("Generate the corrected SQL query."),
        ];

        let response = self.client
            .chat_with_format(
                self.model,
                &messages,
                Some(0.1), // Low temperature for consistent correction
                None,
                None,
            )
            .await?;

        // Extract SQL from response
        self.extract_sql(&response)
    }

    /// Format schema with focus on tables/columns mentioned in error
    fn format_schema_for_error(&self, schema: &Schema, error_message: &str) -> String {
        let mut output = String::new();

        for table in &schema.tables {
            output.push_str(&format!("\n{}:\n", table.name));

            for col in &table.columns {
                let nullable = if col.is_nullable { "NULL" } else { "NOT NULL" };
                let pk = if col.is_primary_key { " [PK]" } else { "" };
                let fk = if col.is_foreign_key {
                    format!(
                        " [FK -> {}.{}]",
                        col.foreign_key_table.as_ref().unwrap_or(&"?".to_string()),
                        col.foreign_key_column.as_ref().unwrap_or(&"?".to_string())
                    )
                } else {
                    String::new()
                };

                // Highlight columns that might be related to the error
                let highlight = if error_message.to_lowercase().contains(&col.name.to_lowercase()) {
                    " <-- CHECK THIS"
                } else {
                    ""
                };

                output.push_str(&format!(
                    "  - {} ({}) {}{}{}{}\n",
                    col.name, col.data_type, nullable, pk, fk, highlight
                ));
            }
        }

        output
    }

    /// Extract SQL from LLM response
    fn extract_sql(&self, response: &str) -> AppResult<String> {
        // Try to find SQL in code blocks
        if let Some(start) = response.find("```sql") {
            if let Some(end) = response[start..].find("```\n").or_else(|| response[start..].rfind("```")) {
                let sql_start = start + 6;
                let actual_end = start + end;
                if actual_end > sql_start {
                    return Ok(response[sql_start..actual_end].trim().to_string());
                }
            }
        }

        // Try plain code blocks
        if let Some(start) = response.find("```") {
            let after_start = start + 3;
            if let Some(end) = response[after_start..].find("```") {
                let sql = response[after_start..after_start + end].trim();
                // Skip language identifier if present
                if let Some(newline) = sql.find('\n') {
                    let first_line = &sql[..newline];
                    if !first_line.to_uppercase().starts_with("SELECT") {
                        return Ok(sql[newline..].trim().to_string());
                    }
                }
                return Ok(sql.to_string());
            }
        }

        // Try to find raw SELECT statement
        if let Some(start) = response.to_uppercase().find("SELECT") {
            let sql_part = &response[start..];
            // Find the end (semicolon or end of string)
            let end = sql_part.find(';').unwrap_or(sql_part.len());
            return Ok(sql_part[..end].trim().to_string());
        }

        // Return the whole response as a last resort
        Ok(response.trim().to_string())
    }
}
