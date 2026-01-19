use super::selector::SelectorAgent;
use super::decomposer::{DecomposerAgent, QueryComplexity};
use super::refiner::{RefinerAgent, RefinerResult};
use super::state::*;
use crate::ai::classification;
use crate::ai::openrouter::OpenRouterClient;
use crate::ai::visualization::generate_plotly_code;
use crate::db::connection::{ConnectionManager, DatabaseType};
use crate::db::query::QueryResult;
use crate::db::schema::{self, Schema};
use crate::error::AppResult;
use crate::storage::AppSettings;
use tauri::{AppHandle, Emitter};

/// Run the MAC-SQL multi-agent pipeline
///
/// Pipeline stages:
/// 1. Selector: Prune schema to relevant tables/columns
/// 2. Decomposer: Judge complexity and generate SQL
/// 3. Refiner: Validate, execute, and self-correct SQL
pub async fn run_mac_sql_agent(
    session_id: String,
    connection_id: String,
    question: String,
    previous_messages: Vec<Message>,
    app: &AppHandle,
    connections: &ConnectionManager,
    settings: &AppSettings,
) -> AppResult<AgentResponse> {
    let client = OpenRouterClient::new(settings.openrouter_api_key.clone());
    let model = &settings.text_to_sql_model;

    // Emit starting message
    emit_thinking(app, &session_id, "Analyzing your question...\n").await?;

    // Step 1: Classify the question
    let question_type = classification::classify_question(
        &question,
        &client,
        model,
    ).await?;

    // For general questions, skip the pipeline and respond directly
    if matches!(question_type, QuestionType::General) {
        return handle_general_question(
            session_id,
            question,
            previous_messages,
            &client,
            model,
            connections,
            &connection_id,
            app,
        ).await;
    }

    // Get full schema
    let full_schema = schema::get_schema(connections, &connection_id, app).await?;
    let conn = connections.get_connection(&connection_id)?;
    let db_type = get_db_type_str(&conn.database_type);

    // Step 2: Selector Agent - Prune schema
    emit_thinking(app, &session_id, "Identifying relevant tables...\n").await?;

    let selector = SelectorAgent::new(&client, model);
    let selector_result = selector.select_relevant_schema(&question, &full_schema).await?;

    emit_thinking(
        app,
        &session_id,
        &format!(
            "Selected tables: {}\n",
            selector_result.selected_tables.join(", ")
        ),
    ).await?;

    // Step 3: Decomposer Agent - Generate SQL
    emit_thinking(app, &session_id, "Generating SQL query...\n").await?;

    let decomposer = DecomposerAgent::new(&client, model);
    let decomposer_result = decomposer.decompose(
        &question,
        &selector_result.pruned_schema,
        &question_type,
        db_type,
        &previous_messages,
    ).await?;

    // Log complexity
    let complexity_msg = match decomposer_result.complexity {
        QueryComplexity::Simple => "Single query generated",
        QueryComplexity::Complex => &format!(
            "Complex query decomposed into {} steps",
            decomposer_result.queries.len()
        ),
    };
    emit_thinking(app, &session_id, &format!("{}\n", complexity_msg)).await?;

    // Step 4: Refiner Agent - Execute and validate each query
    let refiner = RefinerAgent::new(&client, model);
    let mut all_results: Vec<QueryResult> = Vec::new();
    let mut all_sql: Vec<String> = Vec::new();
    let mut refiner_results: Vec<RefinerResult> = Vec::new();

    for (idx, sub_query) in decomposer_result.queries.iter().enumerate() {
        emit_thinking(
            app,
            &session_id,
            &format!("Executing SQL: {}\n", sub_query.sql),
        ).await?;

        // Refine and execute the query
        match refiner.refine_and_execute(
            &sub_query.sql,
            &sub_query.question,
            &selector_result.pruned_schema,
            db_type,
            &connection_id,
            connections,
        ).await {
            Ok(result) => {
                // Emit results
                if result.attempts > 1 {
                    emit_thinking(
                        app,
                        &session_id,
                        &format!("Query succeeded after {} refinement(s)\n", result.attempts),
                    ).await?;
                }

                all_sql.push(result.final_sql.clone());

                // Emit data to frontend
                emit_query_results(
                    app,
                    &session_id,
                    &question_type,
                    &result.result,
                    &question,
                ).await?;

                all_results.push(result.result.clone());
                refiner_results.push(result);
            }
            Err(e) => {
                // Query failed after all refinement attempts
                emit_thinking(
                    app,
                    &session_id,
                    &format!("Query failed: {}\n", e),
                ).await?;

                // If this was a required query, we need to handle the failure
                if idx == 0 || sub_query.depends_on_previous {
                    // Generate a helpful error response
                    let answer = format!(
                        "I encountered an error executing the query: {}\n\n\
                        The query I tried was:\n```sql\n{}\n```\n\n\
                        Please check that the table and column names are correct, \
                        or try rephrasing your question.",
                        e, sub_query.sql
                    );

                    emit_complete(app, &session_id, &answer).await?;

                    return Ok(AgentResponse {
                        answer,
                        sql_queries: vec![sub_query.sql.clone()],
                        iterations: 1,
                    });
                }
            }
        }
    }

    // Step 5: Generate final answer
    let answer = generate_final_answer(
        &question,
        &all_results,
        &decomposer_result.reasoning,
        &client,
        model,
    ).await?;

    emit_token(app, &session_id, &answer).await?;
    emit_complete(app, &session_id, &answer).await?;

    Ok(AgentResponse {
        answer,
        sql_queries: all_sql,
        iterations: refiner_results.iter().map(|r| r.attempts as u8).sum(),
    })
}

/// Handle general (non-data) questions
async fn handle_general_question(
    session_id: String,
    question: String,
    previous_messages: Vec<Message>,
    client: &OpenRouterClient,
    model: &str,
    connections: &ConnectionManager,
    connection_id: &str,
    app: &AppHandle,
) -> AppResult<AgentResponse> {
    // Get schema for context (for schema-related questions)
    let schema = schema::get_schema(connections, connection_id, app).await?;
    let conn = connections.get_connection(connection_id)?;
    let schema_str = format_schema_for_general(&schema, &conn.database_type);

    let system_prompt = format!(
        r#"You are a helpful database assistant. The user has a general question.

DATABASE SCHEMA (for reference):
{}

If they're asking about the database structure, tables, or columns, answer based on the schema above.
If they're greeting you, respond warmly and let them know you can help them query their data.
If they want to know what you can do, explain you can:
- Query and analyze their database
- Generate visualizations from data
- Help them understand their data structure

Keep responses concise and helpful."#,
        schema_str
    );

    let mut messages = vec![Message::system(system_prompt)];
    messages.extend(previous_messages);
    messages.push(Message::user(&question));

    let response = client
        .chat_with_format(model, &messages, Some(0.7), None, None)
        .await?;

    emit_token(app, &session_id, &response).await?;
    emit_complete(app, &session_id, &response).await?;

    Ok(AgentResponse {
        answer: response,
        sql_queries: vec![],
        iterations: 1,
    })
}

/// Generate a final answer summarizing the query results
async fn generate_final_answer(
    question: &str,
    results: &[QueryResult],
    reasoning: &str,
    client: &OpenRouterClient,
    model: &str,
) -> AppResult<String> {
    if results.is_empty() {
        return Ok("No data was retrieved to answer your question.".to_string());
    }

    // For simple single-result queries, we can provide a brief summary
    if results.len() == 1 {
        let result = &results[0];

        if result.row_count == 0 {
            return Ok("The query returned no results matching your criteria.".to_string());
        }

        if result.row_count == 1 && result.columns.len() == 1 {
            // Single value result
            if let Some(row) = result.rows.first() {
                if let Some(value) = row.values().next() {
                    return Ok(format!(
                        "Based on your query, the answer is: **{}**",
                        format_value(value)
                    ));
                }
            }
        }

        // For table results, provide a summary
        return Ok(format!(
            "Found {} row(s) of data. The results are displayed in the table above.",
            result.row_count
        ));
    }

    // For complex multi-query results, use LLM to summarize
    let results_summary: Vec<String> = results.iter().enumerate().map(|(i, r)| {
        format!(
            "Query {}: {} rows, columns: {}",
            i + 1,
            r.row_count,
            r.columns.join(", ")
        )
    }).collect();

    let system_prompt = format!(
        r#"You are summarizing query results. Be concise.

ORIGINAL QUESTION: {}

ANALYSIS: {}

RESULTS:
{}

Provide a brief, clear answer to the user's question based on the data retrieved.
The actual data tables are already displayed, so focus on insights and summary."#,
        question,
        reasoning,
        results_summary.join("\n")
    );

    let messages = vec![
        Message::system(system_prompt),
        Message::user("Summarize the results."),
    ];

    client.chat_with_format(model, &messages, Some(0.3), None, None).await
}

/// Format a JSON value for display
fn format_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        _ => value.to_string(),
    }
}

/// Emit query results to the frontend
async fn emit_query_results(
    app: &AppHandle,
    session_id: &str,
    question_type: &QuestionType,
    data: &QueryResult,
    question: &str,
) -> AppResult<()> {
    let should_emit_table = should_show_table(question_type, data);
    let should_emit_chart = should_show_chart(question_type, data);

    if should_emit_table {
        app.emit(
            "ai_table_data",
            serde_json::json!({
                "session_id": session_id,
                "data": data,
            }),
        )?;
    }

    if should_emit_chart {
        // Generate Plotly visualization data as JSON
        match generate_plotly_code(data, question_type, question) {
            Ok(plotly_viz) => {
                app.emit(
                    "ai_plotly_chart",
                    serde_json::json!({
                        "session_id": session_id,
                        "plotly_data": plotly_viz.data,
                        "plotly_layout": plotly_viz.layout,
                        "title": plotly_viz.title,
                        "chart_type": plotly_viz.chart_type,
                    }),
                )?;
            }
            Err(e) => {
                eprintln!("Chart generation failed: {:?}", e);
            }
        }
    }

    Ok(())
}

/// Determine if table should be shown
fn should_show_table(question_type: &QuestionType, data: &QueryResult) -> bool {
    match question_type {
        QuestionType::TableView => true,
        QuestionType::Statistic => !(data.row_count == 1 && data.columns.len() == 1),
        QuestionType::TemporalChart | QuestionType::CategoryChart => {
            data.row_count > 1 || data.columns.len() > 2
        }
        QuestionType::Complex => true,
        QuestionType::General => false,
    }
}

/// Determine if chart should be shown
fn should_show_chart(question_type: &QuestionType, data: &QueryResult) -> bool {
    match question_type {
        QuestionType::TemporalChart | QuestionType::CategoryChart => data.row_count > 1,
        QuestionType::Statistic => false, // Single values don't need charts
        QuestionType::TableView => false,
        QuestionType::Complex => data.row_count > 1 && data.columns.len() >= 2,
        QuestionType::General => false,
    }
}

/// Emit a token to the frontend (final answer content)
async fn emit_token(app: &AppHandle, session_id: &str, content: &str) -> AppResult<()> {
    app.emit(
        "ai_token",
        serde_json::json!({
            "session_id": session_id,
            "content": content,
        }),
    )?;
    Ok(())
}

/// Emit a thinking token to the frontend (pipeline status)
async fn emit_thinking(app: &AppHandle, session_id: &str, content: &str) -> AppResult<()> {
    app.emit(
        "ai_thinking",
        serde_json::json!({
            "session_id": session_id,
            "content": content,
        }),
    )?;
    Ok(())
}

/// Emit completion event
async fn emit_complete(app: &AppHandle, session_id: &str, answer: &str) -> AppResult<()> {
    app.emit(
        "ai_complete",
        serde_json::json!({
            "session_id": session_id,
            "answer": answer,
        }),
    )?;
    Ok(())
}

/// Get database type string
fn get_db_type_str(db_type: &DatabaseType) -> &'static str {
    match db_type {
        DatabaseType::PostgreSQL => "postgres",
        DatabaseType::MySQL => "mysql",
        DatabaseType::MariaDB => "mariadb",
    }
}

/// Format schema for general questions
fn format_schema_for_general(schema: &Schema, db_type: &DatabaseType) -> String {
    let db_name = db_type.display_name();
    let mut output = format!(
        "Database: {} (Type: {})\n\nTables:\n",
        schema.database_name, db_name
    );

    for table in &schema.tables {
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
