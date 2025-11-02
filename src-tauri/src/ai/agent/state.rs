use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::db::query::QueryResult;

/// Question type classification for routing and prompt selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    General,         // Greetings, pleasantries, non-data questions
    TableView,       // "show me users"
    TemporalChart,   // "users joined over time"
    CategoryChart,   // "users by country"
    Statistic,       // "how many users"
    Complex,         // Multi-step analysis
}


/// Message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<crate::ai::openrouter::types::ToolCall>>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn tool(content: impl Into<String>, tool_call_id: String) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: Some(tool_call_id),
            tool_calls: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Result from tool execution
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub observation: String,
    pub data: Option<QueryResult>,
}

/// Available tools for the agent
#[derive(Debug, Clone)]
pub enum Tool {
    ExecuteSql { query: String },
}

/// Final response from the agent
#[derive(Debug, Serialize)]
pub struct AgentResponse {
    pub answer: String,
    pub sql_queries: Vec<String>,
    pub iterations: u8,
}
