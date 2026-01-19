use crate::ai::openrouter::OpenRouterClient;
use crate::ai::agent::Message;
use crate::db::schema::{Schema, Table, ColumnInfo};
use crate::error::{AppError, AppResult};

/// Result from the Selector Agent
#[derive(Debug, Clone)]
pub struct SelectorResult {
    /// Pruned schema containing only relevant tables/columns
    pub pruned_schema: Schema,
    /// Tables that were selected as relevant
    pub selected_tables: Vec<String>,
}

/// Selector Agent: Prunes the database schema to only relevant tables and columns
///
/// This is the first stage of the MAC-SQL pipeline. It reduces noise and token usage
/// by identifying only the tables and columns needed to answer the user's question.
pub struct SelectorAgent<'a> {
    client: &'a OpenRouterClient,
    model: &'a str,
}

impl<'a> SelectorAgent<'a> {
    pub fn new(client: &'a OpenRouterClient, model: &'a str) -> Self {
        Self { client, model }
    }

    /// Run the selector agent to prune the schema
    pub async fn select_relevant_schema(
        &self,
        question: &str,
        full_schema: &Schema,
    ) -> AppResult<SelectorResult> {
        // Build the prompt for schema selection
        let schema_summary = self.build_schema_summary(full_schema);

        let system_prompt = format!(
            r#"You are a database schema analyst. Your task is to identify which tables and columns are relevant to answer a user's question.

DATABASE SCHEMA:
{}

INSTRUCTIONS:
1. Analyze the user's question carefully
2. Identify ALL tables that could be needed to answer the question
3. For each table, identify the specific columns that are relevant
4. Include tables needed for JOINs even if not directly mentioned
5. Include foreign key columns needed for relationships

IMPORTANT:
- Be inclusive rather than exclusive - it's better to include a potentially relevant table than miss one
- Consider implicit relationships (e.g., "customers" might need "orders" table)
- Include primary and foreign key columns for joins

Respond in this exact JSON format:
{{
    "reasoning": "Brief explanation of why these tables/columns are needed",
    "tables": [
        {{
            "name": "table_name",
            "columns": ["col1", "col2", "col3"]
        }}
    ]
}}"#,
            schema_summary
        );

        let messages = vec![
            Message::system(system_prompt),
            Message::user(question),
        ];

        // Call LLM for schema selection
        let response = self.client
            .chat_with_format(
                self.model,
                &messages,
                Some(0.1), // Low temperature for consistent selection
                None,
                None,
            )
            .await?;

        // Parse the response
        self.parse_selection_response(&response, full_schema)
    }

    /// Build a compact schema summary for the LLM
    fn build_schema_summary(&self, schema: &Schema) -> String {
        let mut output = String::new();

        for table in &schema.tables {
            output.push_str(&format!("\n{}:\n", table.name));

            for col in &table.columns {
                let markers = self.column_markers(col);
                output.push_str(&format!("  - {} ({}){}\n", col.name, col.data_type, markers));
            }
        }

        output
    }

    /// Build column markers (PK, FK, etc.)
    fn column_markers(&self, col: &ColumnInfo) -> String {
        let mut markers = Vec::new();

        if col.is_primary_key {
            markers.push("PK".to_string());
        }
        if col.is_foreign_key {
            if let (Some(ref_table), Some(ref_col)) = (&col.foreign_key_table, &col.foreign_key_column) {
                markers.push(format!("FK->{}.{}", ref_table, ref_col));
            } else {
                markers.push("FK".to_string());
            }
        }

        if markers.is_empty() {
            String::new()
        } else {
            format!(" [{}]", markers.join(", "))
        }
    }

    /// Parse the LLM response and build the pruned schema
    fn parse_selection_response(
        &self,
        response: &str,
        full_schema: &Schema,
    ) -> AppResult<SelectorResult> {
        // Extract JSON from response (handle markdown code blocks)
        let json_str = self.extract_json(response);

        // Parse the JSON response
        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| AppError::AgentError(format!("Failed to parse selector response: {}. Response: {}", e, response)))?;

        let tables_array = parsed["tables"]
            .as_array()
            .ok_or_else(|| AppError::AgentError("Invalid selector response: missing tables array".into()))?;

        // Build the pruned schema
        let mut pruned_tables = Vec::new();
        let mut selected_table_names = Vec::new();

        for table_obj in tables_array {
            let table_name = table_obj["name"]
                .as_str()
                .ok_or_else(|| AppError::AgentError("Invalid table object: missing name".into()))?;

            // Find the table in the full schema
            if let Some(full_table) = full_schema.tables.iter().find(|t| t.name.eq_ignore_ascii_case(table_name)) {
                let column_names: Vec<String> = table_obj["columns"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_else(|| {
                        // If no columns specified, include all columns
                        full_table.columns.iter().map(|c| c.name.clone()).collect()
                    });

                // Filter columns based on selection (case-insensitive)
                let filtered_columns: Vec<ColumnInfo> = if column_names.is_empty() {
                    full_table.columns.clone()
                } else {
                    full_table.columns
                        .iter()
                        .filter(|c| {
                            column_names.iter().any(|cn| cn.eq_ignore_ascii_case(&c.name))
                        })
                        .cloned()
                        .collect()
                };

                // Always include primary key and foreign key columns even if not selected
                let mut final_columns = filtered_columns;
                for col in &full_table.columns {
                    if (col.is_primary_key || col.is_foreign_key)
                        && !final_columns.iter().any(|c| c.name == col.name)
                    {
                        final_columns.push(col.clone());
                    }
                }

                pruned_tables.push(Table {
                    name: full_table.name.clone(),
                    schema: full_table.schema.clone(),
                    row_count: full_table.row_count,
                    columns: final_columns,
                    indexes: full_table.indexes.clone(),
                    triggers: full_table.triggers.clone(),
                    constraints: full_table.constraints.clone(),
                });

                selected_table_names.push(full_table.name.clone());
            }
        }

        // If no tables were selected, fall back to full schema
        if pruned_tables.is_empty() {
            return Ok(SelectorResult {
                pruned_schema: full_schema.clone(),
                selected_tables: full_schema.tables.iter().map(|t| t.name.clone()).collect(),
            });
        }

        Ok(SelectorResult {
            pruned_schema: Schema {
                database_name: full_schema.database_name.clone(),
                tables: pruned_tables,
            },
            selected_tables: selected_table_names,
        })
    }

    /// Extract JSON from a response that might contain markdown code blocks
    fn extract_json(&self, response: &str) -> String {
        // Try to find JSON in code blocks first
        if let Some(start) = response.find("```json") {
            if let Some(end) = response[start..].find("```\n").or_else(|| response[start..].rfind("```")) {
                let json_start = start + 7; // Length of "```json"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_code_block() {
        let client = OpenRouterClient::new("test".to_string());
        let agent = SelectorAgent::new(&client, "test-model");

        let response = r#"Here is my analysis:

```json
{
    "reasoning": "test",
    "tables": []
}
```

That's the result."#;

        let json = agent.extract_json(response);
        assert!(json.contains("reasoning"));
    }
}
