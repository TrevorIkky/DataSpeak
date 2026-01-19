use crate::ai::openrouter::OpenRouterClient;
use crate::ai::agent::{Message, MessageRole, QuestionType};
use crate::db::schema::Schema;
use crate::error::{AppError, AppResult};

/// Complexity level of a question
#[derive(Debug, Clone, PartialEq)]
pub enum QueryComplexity {
    /// Simple query - can be answered with a single SQL statement
    Simple,
    /// Complex query - requires decomposition into sub-queries
    Complex,
}

/// A sub-query generated from decomposition
#[derive(Debug, Clone)]
pub struct SubQuery {
    /// The sub-question in natural language
    pub question: String,
    /// The SQL query to answer this sub-question
    pub sql: String,
    /// Order in which this should be executed (0-indexed)
    pub order: usize,
    /// Whether this depends on previous query results
    pub depends_on_previous: bool,
}

/// Result from the Decomposer Agent
#[derive(Debug, Clone)]
pub struct DecomposerResult {
    /// Complexity assessment
    pub complexity: QueryComplexity,
    /// Generated SQL queries (single for simple, multiple for complex)
    pub queries: Vec<SubQuery>,
    /// Chain of thought reasoning
    pub reasoning: String,
}

/// Decomposer Agent: Judges query complexity and generates SQL
///
/// This is the second stage of the MAC-SQL pipeline. It:
/// 1. Assesses if the question is simple or complex
/// 2. For simple questions: generates SQL directly
/// 3. For complex questions: breaks into sub-problems and generates progressive SQL
pub struct DecomposerAgent<'a> {
    client: &'a OpenRouterClient,
    model: &'a str,
}

impl<'a> DecomposerAgent<'a> {
    pub fn new(client: &'a OpenRouterClient, model: &'a str) -> Self {
        Self { client, model }
    }

    /// Decompose the question and generate SQL queries
    pub async fn decompose(
        &self,
        question: &str,
        schema: &Schema,
        question_type: &QuestionType,
        db_type: &str,
        conversation_history: &[Message],
    ) -> AppResult<DecomposerResult> {
        let schema_str = self.format_schema(schema, db_type);
        let history_str = self.format_conversation_history(conversation_history);

        let system_prompt = format!(
            r#"You are an expert SQL analyst. Your task is to analyze a user's question and generate the SQL needed to answer it.

DATABASE SCHEMA:
{}

DATABASE TYPE: {} (use {}-compatible SQL syntax)
{}
PROCESS:
1. First, assess the question complexity:
   - SIMPLE: Can be answered with a single SQL query (most questions)
   - COMPLEX: Requires multiple queries or sub-queries (rare, only for multi-step analysis)

2. For SIMPLE questions:
   - Generate a single, complete SQL query
   - Use JOINs, aggregations, and subqueries within the single statement

3. For COMPLEX questions:
   - Break down into sequential steps
   - Each step should build on previous results
   - Generate SQL for each step

RULES:
- Only SELECT queries (no INSERT, UPDATE, DELETE, etc.)
- Always include LIMIT clause (max 100 rows)
- Use proper {} SQL syntax
- Prefer CTEs (WITH clause) for complex logic in a single query
- Only mark as COMPLEX if truly requiring multiple separate queries
- If the user refers to "that", "those", "it", etc., use the CONVERSATION HISTORY to understand what they mean

Respond in this exact JSON format:
{{
    "complexity": "simple" or "complex",
    "reasoning": "Your chain of thought explaining how to answer this question",
    "queries": [
        {{
            "question": "The sub-question this query answers",
            "sql": "SELECT ... FROM ... LIMIT 100",
            "order": 0,
            "depends_on_previous": false
        }}
    ]
}}"#,
            schema_str, db_type, db_type, history_str, db_type
        );

        // Add context about question type
        let context = match question_type {
            QuestionType::Statistic => "\n\nNote: This question asks for a specific metric or count. Use aggregate functions.",
            QuestionType::TemporalChart => "\n\nNote: This question involves time-series data. Include date grouping and ordering.",
            QuestionType::CategoryChart => "\n\nNote: This question involves categories. Use GROUP BY for grouping.",
            QuestionType::TableView => "\n\nNote: User wants to view table data. Simple SELECT with appropriate columns.",
            QuestionType::Complex => "\n\nNote: This has been classified as a complex analytical question.",
            QuestionType::General => "",
        };

        let messages = vec![
            Message::system(format!("{}{}", system_prompt, context)),
            Message::user(question),
        ];

        let response = self.client
            .chat_with_format(
                self.model,
                &messages,
                Some(0.2), // Slightly higher temperature for creative SQL
                None,
                None,
            )
            .await?;

        self.parse_decomposer_response(&response)
    }

    /// Format conversation history for context
    fn format_conversation_history(&self, history: &[Message]) -> String {
        if history.is_empty() {
            return String::new();
        }

        let mut output = String::from("\nCONVERSATION HISTORY:\n");

        // Only include the last 5 exchanges to keep context manageable
        let recent_history: Vec<_> = history.iter()
            .rev()
            .take(10) // 5 exchanges = 10 messages (user + assistant pairs)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        for msg in recent_history {
            let role = match msg.role {
                MessageRole::User => "User",
                MessageRole::Assistant => "Assistant",
                _ => continue,
            };
            // Truncate long messages to save tokens
            let content = if msg.content.len() > 200 {
                format!("{}...", &msg.content[..200])
            } else {
                msg.content.clone()
            };
            output.push_str(&format!("{}: {}\n", role, content));
        }

        output.push('\n');
        output
    }

    /// Format schema for the decomposer prompt
    fn format_schema(&self, schema: &Schema, db_type: &str) -> String {
        let mut output = format!("Database: {} (Type: {})\n\nTables:\n", schema.database_name, db_type);

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

                output.push_str(&format!(
                    "  - {} ({}) {}{}{}\n",
                    col.name, col.data_type, nullable, pk, fk
                ));
            }
        }

        output
    }

    /// Parse the LLM response into DecomposerResult
    fn parse_decomposer_response(&self, response: &str) -> AppResult<DecomposerResult> {
        let json_str = self.extract_json(response);

        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| AppError::AgentError(format!("Failed to parse decomposer response: {}. Response: {}", e, response)))?;

        let complexity_str = parsed["complexity"]
            .as_str()
            .unwrap_or("simple")
            .to_lowercase();

        let complexity = if complexity_str == "complex" {
            QueryComplexity::Complex
        } else {
            QueryComplexity::Simple
        };

        let reasoning = parsed["reasoning"]
            .as_str()
            .unwrap_or("No reasoning provided")
            .to_string();

        let queries_array = parsed["queries"]
            .as_array()
            .ok_or_else(|| AppError::AgentError("Invalid decomposer response: missing queries array".into()))?;

        let mut queries = Vec::new();

        for query_obj in queries_array {
            let question = query_obj["question"]
                .as_str()
                .unwrap_or("Answer the user's question")
                .to_string();

            let sql = query_obj["sql"]
                .as_str()
                .ok_or_else(|| AppError::AgentError("Invalid query object: missing sql".into()))?
                .to_string();

            let order = query_obj["order"]
                .as_u64()
                .unwrap_or(0) as usize;

            let depends_on_previous = query_obj["depends_on_previous"]
                .as_bool()
                .unwrap_or(false);

            queries.push(SubQuery {
                question,
                sql,
                order,
                depends_on_previous,
            });
        }

        // Ensure queries are sorted by order
        queries.sort_by_key(|q| q.order);

        // If no queries were generated, this is an error
        if queries.is_empty() {
            return Err(AppError::AgentError("Decomposer generated no queries".into()));
        }

        Ok(DecomposerResult {
            complexity,
            queries,
            reasoning,
        })
    }

    /// Extract JSON from response (handles markdown code blocks)
    fn extract_json(&self, response: &str) -> String {
        // Try to find JSON in code blocks first
        if let Some(start) = response.find("```json") {
            if let Some(end) = response[start..].find("```\n").or_else(|| response[start..].rfind("```")) {
                let json_start = start + 7;
                let actual_end = start + end;
                if actual_end > json_start {
                    return response[json_start..actual_end].trim().to_string();
                }
            }
        }

        // Try plain code blocks
        if let Some(start) = response.find("```") {
            let after_start = start + 3;
            if let Some(end) = response[after_start..].find("```") {
                return response[after_start..after_start + end].trim().to_string();
            }
        }

        // Try to find raw JSON
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                if end > start {
                    return response[start..=end].to_string();
                }
            }
        }

        response.trim().to_string()
    }
}
