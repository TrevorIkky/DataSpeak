use serde::{Serialize, Deserialize};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Query error: {0}")]
    QueryError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("AI API error: {0}")]
    AiError(String),

    #[error("OpenRouter API error: {0}")]
    OpenRouterError(String),

    #[error("Agent error: {0}")]
    AgentError(String),

    #[error("Security error: {0}")]
    SecurityError(String),

    #[error("Visualization error: {0}")]
    VisualizationError(String),

    #[error("Import/Export error: {0}")]
    ImportExportError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("{0}")]
    Other(String),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::SerializationError(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::AiError(err.to_string())
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(err: zip::result::ZipError) -> Self {
        AppError::ImportExportError(err.to_string())
    }
}

impl From<csv::Error> for AppError {
    fn from(err: csv::Error) -> Self {
        AppError::ImportExportError(err.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(err: tauri::Error) -> Self {
        AppError::Other(err.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
