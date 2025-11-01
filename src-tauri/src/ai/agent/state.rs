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

/// Agent state maintaining conversation context and execution status
#[derive(Debug, Clone)]
pub struct AgentState {
    pub session_id: String,
    pub connection_id: String,
    pub question: String,
    pub question_type: QuestionType,
    pub schema: String,
    pub messages: Vec<Message>,
    pub iterations: u8,
    pub max_iterations: u8,
    pub tool_results: Vec<ToolResult>,
}

impl AgentState {
    pub fn new(
        session_id: String,
        connection_id: String,
        question: String,
        question_type: QuestionType,
        schema: String,
    ) -> Self {
        Self {
            session_id,
            connection_id,
            question,
            question_type,
            schema,
            messages: Vec::new(),
            iterations: 0,
            max_iterations: 5,
            tool_results: Vec::new(),
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn add_tool_result(&mut self, result: ToolResult) {
        self.tool_results.push(result);
    }

    pub fn increment_iteration(&mut self) {
        self.iterations += 1;
    }

    pub fn has_reached_max_iterations(&self) -> bool {
        self.iterations >= self.max_iterations
    }
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
    pub execution_time_ms: u128,
}

/// Decision from routing node
#[derive(Debug)]
pub enum Decision {
    FinalAnswer(String),
    ToolCall(ToolCall),
    Continue,
    Error(String),
}

/// Parsed tool call from LLM response
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub tool: Tool,
    pub raw_response: String,
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
