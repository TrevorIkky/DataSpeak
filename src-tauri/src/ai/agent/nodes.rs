use super::state::*;
use crate::ai::openrouter::OpenRouterClient;
use crate::ai::tools;
use crate::ai::visualization;
use crate::db::connection::ConnectionManager;
use crate::error::AppResult;
use crate::storage::AppSettings;
use futures::StreamExt;
use tauri::{AppHandle, Emitter};

/// Node 1: Think - Call LLM and stream response
pub async fn think_node(
    state: &AgentState,
    app: &AppHandle,
    client: &OpenRouterClient,
    settings: &AppSettings,
) -> AppResult<String> {
    let mut full_response = String::new();

    let mut stream = client
        .chat_stream(&settings.text_to_sql_model, &state.messages, Some(0.1))
        .await?;

    while let Some(token_result) = stream.next().await {
        let token = token_result?;

        // Emit token to frontend
        app.emit(
            "ai_token",
            serde_json::json!({
                "session_id": state.session_id,
                "content": token,
            }),
        )?;

        full_response.push_str(&token);
    }

    Ok(full_response)
}

/// Node 2: Route - Decide what to do next
pub fn route_node(response: &str) -> Decision {
    // Check for final answer
    if response.contains("Final Answer:") {
        match tools::extract_final_answer(response) {
            Ok(answer) => return Decision::FinalAnswer(answer),
            Err(e) => return Decision::Error(e.to_string()),
        }
    }

    // Check for tool call
    if response.contains("Action:") {
        match tools::parse(response) {
            Ok(tool_call) => return Decision::ToolCall(tool_call),
            Err(e) => return Decision::Error(format!("Failed to parse tool call: {}", e)),
        }
    }

    Decision::Continue
}

/// Node 3: Act - Execute tool
pub async fn act_node(
    tool_call: &ToolCall,
    connection_id: &str,
    connections: &ConnectionManager,
) -> AppResult<ToolResult> {
    tools::execute_sql_tool(&tool_call.tool, connection_id, connections).await
}

/// Node 4: Analyze result and emit appropriate events
pub async fn analyze_and_emit(
    app: &AppHandle,
    state: &AgentState,
    tool_result: &ToolResult,
) -> AppResult<()> {
    let data = match &tool_result.data {
        Some(d) => d,
        None => return Ok(()), // No data to analyze
    };

    match &state.question_type {
        QuestionType::TableView => {
            app.emit(
                "ai_table_data",
                serde_json::json!({
                    "session_id": state.session_id,
                    "data": data,
                }),
            )?;
        }

        QuestionType::TemporalChart | QuestionType::CategoryChart => {
            // Generate chart config
            let chart_config = visualization::generate_config(data, &state.question_type)?;

            app.emit(
                "ai_chart_data",
                serde_json::json!({
                    "session_id": state.session_id,
                    "config": chart_config,
                    "data": data,
                }),
            )?;
        }

        QuestionType::Statistic => {
            // Extract single value
            if let Some(first_row) = data.rows.first() {
                if let Some((_, value)) = first_row.iter().next() {
                    app.emit(
                        "ai_statistic",
                        serde_json::json!({
                            "session_id": state.session_id,
                            "value": value,
                            "label": state.question.clone(),
                        }),
                    )?;
                }
            }
        }

        _ => {}
    }

    Ok(())
}

/// Emit completion event
pub async fn emit_complete(
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
