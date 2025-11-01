use crate::ai::agent::Message;
use crate::error::AppResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

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
