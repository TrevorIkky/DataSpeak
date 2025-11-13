use crate::error::{AppError, AppResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const MAX_HISTORY_SIZE: usize = 200;

static HISTORY_PATH: OnceLock<Mutex<PathBuf>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHistoryEntry {
    pub id: String,
    pub query: String,
    pub connection_id: String,
    pub executed_at: DateTime<Utc>,
    pub execution_time_ms: f64,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryHistory {
    entries: Vec<QueryHistoryEntry>,
}

impl Default for QueryHistory {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

pub fn init_history_path(app_data_dir: PathBuf) {
    let path = app_data_dir.join("query_history.json");
    HISTORY_PATH.set(Mutex::new(path)).ok();
}

fn get_history_path() -> AppResult<PathBuf> {
    HISTORY_PATH
        .get()
        .ok_or_else(|| AppError::StorageError("History path not initialized".to_string()))?
        .lock()
        .map(|p| p.clone())
        .map_err(|e| AppError::StorageError(format!("Failed to lock history path: {}", e)))
}

fn load_history() -> AppResult<QueryHistory> {
    let path = get_history_path()?;

    if !path.exists() {
        return Ok(QueryHistory::default());
    }

    let json = fs::read_to_string(&path)
        .map_err(|e| AppError::StorageError(format!("Failed to read query history: {}", e)))?;
    let history: QueryHistory = serde_json::from_str(&json)
        .map_err(|e| AppError::StorageError(format!("Failed to parse query history: {}", e)))?;

    Ok(history)
}

fn save_history(history: &QueryHistory) -> AppResult<()> {
    let path = get_history_path()?;
    let json = serde_json::to_string_pretty(history)
        .map_err(|e| AppError::StorageError(format!("Failed to serialize query history: {}", e)))?;
    fs::write(&path, json)
        .map_err(|e| AppError::StorageError(format!("Failed to write query history: {}", e)))?;

    Ok(())
}

/// Add a query to history
pub async fn add_query_to_history(
    query: String,
    connection_id: String,
    execution_time_ms: f64,
    success: bool,
) -> AppResult<()> {
    let mut history = load_history()?;

    // Create new entry
    let entry = QueryHistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        query,
        connection_id,
        executed_at: Utc::now(),
        execution_time_ms,
        success,
    };

    // Add to front of list
    history.entries.insert(0, entry);

    // Keep only last 200 entries
    if history.entries.len() > MAX_HISTORY_SIZE {
        history.entries.truncate(MAX_HISTORY_SIZE);
    }

    save_history(&history)?;

    Ok(())
}

/// Get query history for a specific connection
pub async fn get_query_history(connection_id: Option<String>) -> AppResult<Vec<QueryHistoryEntry>> {
    let history = load_history()?;

    if let Some(conn_id) = connection_id {
        // Filter by connection ID
        Ok(history.entries.into_iter()
            .filter(|entry| entry.connection_id == conn_id)
            .collect())
    } else {
        // Return all entries
        Ok(history.entries)
    }
}

/// Delete a specific query from history by ID
pub async fn delete_query_from_history(query_id: String) -> AppResult<()> {
    let mut history = load_history()?;

    // Remove the entry with the matching ID
    history.entries.retain(|entry| entry.id != query_id);

    save_history(&history)?;

    Ok(())
}

/// Clear query history
pub async fn clear_query_history() -> AppResult<()> {
    let path = get_history_path()?;

    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| AppError::StorageError(format!("Failed to delete query history: {}", e)))?;
    }

    Ok(())
}
