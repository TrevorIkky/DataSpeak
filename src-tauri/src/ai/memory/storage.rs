use crate::ai::agent::{Message, MessageRole};
use crate::error::AppResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Lightweight metadata for listing conversations without full message history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMetadata {
    pub session_id: String,
    pub connection_id: String,
    pub title: String,
    pub message_count: usize,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct ConversationHistory {
    pub session_id: String,
    pub connection_id: String,
    pub messages: Vec<Message>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,
}

/// Save conversation to disk
pub fn save_conversation(
    app: &AppHandle,
    session_id: &str,
    connection_id: &str,
    messages: &[Message],
) -> AppResult<()> {
    let path = get_conversation_path(app, session_id)?;

    let history = ConversationHistory {
        session_id: session_id.to_string(),
        connection_id: connection_id.to_string(),
        messages: messages.to_vec(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let json = serde_json::to_string_pretty(&history)?;
    std::fs::write(path, json)?;

    Ok(())
}

/// Load conversation from disk
pub fn load_conversation(app: &AppHandle, session_id: &str) -> AppResult<Vec<Message>> {
    let path = get_conversation_path(app, session_id)?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let json = std::fs::read_to_string(path)?;
    let history: ConversationHistory = serde_json::from_str(&json)?;

    Ok(history.messages)
}

/// Load last N messages from conversation (for context window management)
pub fn load_conversation_with_limit(
    app: &AppHandle,
    session_id: &str,
    limit: usize,
) -> AppResult<Vec<Message>> {
    let all_messages = load_conversation(app, session_id)?;

    if all_messages.len() <= limit {
        return Ok(all_messages);
    }

    // Take the last N messages
    let start_index = all_messages.len() - limit;
    Ok(all_messages[start_index..].to_vec())
}

/// Clear conversation from disk
pub fn clear_conversation(app: &AppHandle, session_id: &str) -> AppResult<()> {
    let path = get_conversation_path(app, session_id)?;

    if path.exists() {
        std::fs::remove_file(path)?;
    }

    Ok(())
}

fn get_conversation_path(app: &AppHandle, session_id: &str) -> AppResult<PathBuf> {
    let app_data = app.path().app_data_dir()?;
    let conv_dir = app_data.join("conversations");
    std::fs::create_dir_all(&conv_dir)?;
    Ok(conv_dir.join(format!("{}.json", session_id)))
}

/// List all conversations for a specific database connection
pub fn list_conversations(
    app: &AppHandle,
    connection_id: &str,
) -> AppResult<Vec<ConversationMetadata>> {
    let app_data = app.path().app_data_dir()?;
    let conv_dir = app_data.join("conversations");

    if !conv_dir.exists() {
        return Ok(Vec::new());
    }

    let mut conversations = Vec::new();

    for entry in std::fs::read_dir(conv_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only process .json files
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(history) = serde_json::from_str::<ConversationHistory>(&json) {
                    // Filter by connection_id
                    if history.connection_id == connection_id {
                        let title = generate_title(&history.messages);
                        conversations.push(ConversationMetadata {
                            session_id: history.session_id,
                            connection_id: history.connection_id,
                            title,
                            message_count: history.messages.len(),
                            created_at: history.created_at,
                            updated_at: history.updated_at,
                        });
                    }
                }
            }
        }
    }

    // Sort by updated_at descending (most recent first)
    conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(conversations)
}

/// Generate a title from the first user message in the conversation
fn generate_title(messages: &[Message]) -> String {
    messages
        .iter()
        .find(|m| matches!(m.role, MessageRole::User))
        .map(|m| {
            let content = m.content.trim();
            if content.len() > 50 {
                format!("{}...", &content[..47])
            } else {
                content.to_string()
            }
        })
        .unwrap_or_else(|| "New conversation".to_string())
}
