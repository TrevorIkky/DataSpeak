use super::state::*;
use crate::ai::classification;
use crate::ai::openrouter::OpenRouterClient;
use crate::ai::prompts;
use crate::ai::tools;
use crate::ai::visualization;
use crate::db::connection::ConnectionManager;
use crate::db::schema;
use crate::error::{AppError, AppResult};
use crate::storage::AppSettings;
use tauri::{AppHandle, Emitter};

/// Run the ReAct agent
pub async fn run_react_agent(
    session_id: String,
    connection_id: String,
    question: String,
    previous_messages: Vec<Message>,
    app: &AppHandle,
    connections: &ConnectionManager,
    settings: &AppSettings,
) -> AppResult<AgentResponse> {
    // Create OpenRouter client
    let client = OpenRouterClient::new(settings.openrouter_api_key.clone());

    // Classify question
    let question_type = classification::classify_question(
        &question,
        &client,
        &settings.text_to_sql_model,
        true, // Use LLM fallback
    )
    .await?;

    // Handle general questions directly (no database access needed)
    if matches!(question_type, QuestionType::General) {
        use futures::StreamExt;

        let system_prompt = prompts::build_system_prompt("", &question_type);
        let mut messages = vec![Message::system(system_prompt)];

        // Add previous conversation history
        messages.extend(previous_messages.clone());

        // Add current question
        messages.push(Message::user(&question));

        let mut stream = client
            .chat_stream(&settings.text_to_sql_model, &messages, Some(0.7))
            .await?;

        let mut answer = String::new();
        while let Some(token_result) = stream.next().await {
            let token = token_result?;

            // Emit token to frontend
            app.emit(
                "ai_token",
                serde_json::json!({
                    "session_id": session_id,
                    "content": token,
                }),
            )?;

            answer.push_str(&token);
        }

        // Extract answer (remove "Final Answer:" prefix if present)
        let answer = answer
            .trim()
            .strip_prefix("Final Answer:")
            .unwrap_or(&answer)
            .trim()
            .to_string();

        // Emit completion
        emit_complete(app, &session_id, &answer).await?;

        return Ok(AgentResponse {
            answer,
            sql_queries: Vec::new(),
            iterations: 0,
        });
    }

    // Get schema and connection info
    let schema_data = schema::get_schema(connections, &connection_id, app).await?;
    let conn = connections.get_connection(&connection_id)?;
    let schema_str = format_schema_for_ai(&schema_data, &conn.database_type);

    // Build tool definitions
    let tool_defs = tools::build_tools();

    // Initialize messages with system prompt
    let system_prompt = prompts::build_system_prompt(&schema_str, &question_type);
    let mut messages = vec![Message::system(system_prompt)];

    // Add previous conversation history
    messages.extend(previous_messages.clone());

    // Add current question
    messages.push(Message::user(&question));

    let mut sql_queries = Vec::new();
    let max_iterations = 5;
    let mut iterations = 0;

    // Native tool calling loop
    while iterations < max_iterations {
        iterations += 1;

        // Call LLM with tools
        let response = client
            .chat_with_tools(&settings.text_to_sql_model, &messages, tool_defs.clone(), Some(0.1))
            .await?;

        let choice = response.choices.first()
            .ok_or_else(|| AppError::AgentError("No response from model".into()))?;

        // Check if model wants to call tools
        if let Some(tool_calls) = &choice.message.tool_calls {
            // Emit thinking content if present (model's reasoning before tool call)
            if let Some(thinking) = &choice.message.content {
                if !thinking.is_empty() {
                    app.emit(
                        "ai_token",
                        serde_json::json!({
                            "session_id": session_id,
                            "content": thinking,
                        }),
                    )?;
                }
            }

            // Add assistant message with tool calls (must be before tool results)
            messages.push(Message {
                role: MessageRole::Assistant,
                content: choice.message.content.clone().unwrap_or_default(),
                timestamp: chrono::Utc::now(),
                tool_call_id: None,
                tool_calls: Some(tool_calls.clone()),
            });

            // Process tool calls
            for tool_call in tool_calls {
                if tool_call.function.name != "execute_sql" {
                    continue;
                }

                // Parse arguments
                let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                    .map_err(|e| AppError::AgentError(format!("Failed to parse tool args: {}", e)))?;

                let query = args["query"].as_str()
                    .ok_or_else(|| AppError::AgentError("Missing query in tool call".into()))?;

                sql_queries.push(query.to_string());

                // Emit execution marker
                app.emit(
                    "ai_token",
                    serde_json::json!({
                        "session_id": session_id,
                        "content": format!("\n\n**Executing SQL:**\n```sql\n{}\n```\n", query),
                    }),
                )?;

                // Execute SQL
                let tool_result = match tools::execute_sql_tool(
                    &crate::ai::agent::Tool::ExecuteSql { query: query.to_string() },
                    &connection_id,
                    connections,
                ).await {
                    Ok(result) => result,
                    Err(e) => {
                        // Tool execution failed - provide helpful error message
                        let error_msg = format!(
                            "SQL execution failed: {}\n\nCommon fixes:\n\
                            - Check for typos in SQL keywords (SELECT, FROM, WHERE, etc.)\n\
                            - Verify table and column names exist in the schema\n\
                            - Ensure LIMIT clause is present (required, max 100)\n\
                            - Check for proper quote usage (single quotes for strings)\n\
                            - Verify JOIN conditions reference valid columns\n\
                            Please review the error, check the schema, and try again with a corrected query.",
                            e
                        );

                        messages.push(Message::tool(error_msg.clone(), tool_call.id.clone()));

                        // Continue to next iteration so model can fix the query
                        continue;
                    }
                };

                // Emit data events if we have query results
                if let Some(data) = &tool_result.data {
                    // Determine what to emit based on question type and data characteristics
                    let should_emit_table = should_show_table(&question_type, data);
                    let should_emit_chart = should_show_chart(&question_type, data);
                    let should_emit_statistic = should_show_statistic(&question_type, data);

                    // Emit table data (conditionally)
                    if should_emit_table {
                        app.emit(
                            "ai_table_data",
                            serde_json::json!({
                                "session_id": session_id,
                                "data": data,
                            }),
                        )?;
                    }

                    // Try to generate visualization (conditionally)
                    if should_emit_chart {
                        if let Ok(viz_config) = visualization::generate_config(data, &question_type) {
                            app.emit(
                                "ai_chart_data",
                                serde_json::json!({
                                    "session_id": session_id,
                                    "config": viz_config,
                                    "data": data,
                                }),
                            )?;
                        }
                    }

                    // Emit statistic (conditionally)
                    if should_emit_statistic {
                        if let Some(first_row) = data.rows.first() {
                            if let Some(first_col) = data.columns.first() {
                                if let Some(value) = first_row.get(first_col) {
                                    app.emit(
                                        "ai_statistic",
                                        serde_json::json!({
                                            "session_id": session_id,
                                            "value": value,
                                            "label": &question,
                                        }),
                                    )?;
                                }
                            }
                        }
                    }
                }

                // Add tool result
                messages.push(Message::tool(tool_result.observation.clone(), tool_call.id.clone()));
            }
        } else if let Some(content) = &choice.message.content {
            // Model returned final answer (no tool calls)
            // Emit final answer
            app.emit(
                "ai_token",
                serde_json::json!({
                    "session_id": session_id,
                    "content": content,
                }),
            )?;

            emit_complete(app, &session_id, content).await?;

            return Ok(AgentResponse {
                answer: content.clone(),
                sql_queries,
                iterations,
            });
        } else {
            return Err(AppError::AgentError("Model returned empty response".into()));
        }
    }

    // Max iterations reached
    Err(AppError::AgentError(format!(
        "Maximum iterations ({}) reached without finding answer",
        max_iterations
    )))
}

/// Format schema data for AI context (compact representation)
fn format_schema_for_ai(schema_data: &schema::Schema, db_type: &crate::db::connection::DatabaseType) -> String {
    let db_name = db_type.display_name();

    let mut output = format!(
        "Database: {} (Type: {})\n\nIMPORTANT: Use {}-compatible SQL syntax.\n\nTables:\n",
        schema_data.database_name,
        db_name,
        db_name
    );

    for table in &schema_data.tables {
        output.push_str(&format!("\n{}:\n", table.name));

        for col in &table.columns {
            let nullable = if col.is_nullable { "NULL" } else { "NOT NULL" };
            let pk = if col.is_primary_key { " PRIMARY KEY" } else { "" };
            let fk = if col.is_foreign_key {
                format!(
                    " -> {}.{}",
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

/// Determine if table should be shown based on question type and data
fn should_show_table(question_type: &QuestionType, data: &crate::db::query::QueryResult) -> bool {
    match question_type {
        // Always show table for explicit table view requests
        QuestionType::TableView => true,

        // For statistics, only show table if multiple rows/columns (not just a single number)
        QuestionType::Statistic => {
            // If it's a single value (1 row, 1 col), don't show table (statistic card is enough)
            !(data.row_count == 1 && data.columns.len() == 1)
        },

        // For temporal/category charts, show table if visualization fails or data is simple
        QuestionType::TemporalChart | QuestionType::CategoryChart => {
            // Show table if we have reasonable amount of data to display
            // Skip if it's a single aggregate value
            data.row_count > 1 || data.columns.len() > 2
        },

        // For complex queries, always show table
        QuestionType::Complex => true,

        // General questions shouldn't have data
        QuestionType::General => false,
    }
}

/// Determine if chart should be shown based on question type and data
fn should_show_chart(question_type: &QuestionType, data: &crate::db::query::QueryResult) -> bool {
    match question_type {
        // Explicit chart requests should attempt visualization
        QuestionType::TemporalChart | QuestionType::CategoryChart => true,

        // For table views, don't auto-generate charts
        QuestionType::TableView => false,

        // For statistics, don't show charts
        QuestionType::Statistic => false,

        // For complex queries, try if we have suitable data (2+ cols, multiple rows)
        QuestionType::Complex => data.row_count > 1 && data.columns.len() >= 2,

        // General questions shouldn't have data
        QuestionType::General => false,
    }
}

/// Determine if statistic card should be shown
fn should_show_statistic(question_type: &QuestionType, data: &crate::db::query::QueryResult) -> bool {
    // Only show statistic card for explicit statistic questions with a single value
    matches!(question_type, QuestionType::Statistic)
        && data.row_count == 1
        && data.columns.len() == 1
}

/// Emit completion event
async fn emit_complete(
    app: &AppHandle,
    session_id: &str,
    answer: &str,
) -> AppResult<()> {
    app.emit(
        "ai_complete",
        serde_json::json!({
            "session_id": session_id,
            "answer": answer,
        }),
    )?;

    Ok(())
}
