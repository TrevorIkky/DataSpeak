use serde::{Deserialize, Serialize};

/// Request to OpenRouter API
#[derive(Debug, Serialize)]
pub struct OpenRouterRequest {
    pub model: String,
    pub messages: Vec<OpenRouterMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}

/// Response format for structured outputs
#[derive(Debug, Serialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<JsonSchema>,
}

/// JSON Schema definition for structured outputs
#[derive(Debug, Serialize)]
pub struct JsonSchema {
    pub name: String,
    pub strict: bool,
    pub schema: serde_json::Value,
}

/// Tool definition for OpenRouter
#[derive(Debug, Serialize, Clone)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

/// Function definition for a tool
#[derive(Debug, Serialize, Clone)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool call from the model
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Message in OpenRouter format
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenRouterMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl From<&crate::ai::agent::Message> for OpenRouterMessage {
    fn from(msg: &crate::ai::agent::Message) -> Self {
        let role = match msg.role {
            crate::ai::agent::MessageRole::System => "system",
            crate::ai::agent::MessageRole::User => "user",
            crate::ai::agent::MessageRole::Assistant => "assistant",
            crate::ai::agent::MessageRole::Tool => "tool",
        };

        Self {
            role: role.to_string(),
            content: Some(msg.content.clone()),
            tool_calls: msg.tool_calls.clone(),
            tool_call_id: msg.tool_call_id.clone(),
        }
    }
}

/// Response from OpenRouter API (non-streaming)
#[derive(Debug, Deserialize)]
pub struct OpenRouterResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: OpenRouterMessage,
    pub finish_reason: Option<String>,
}

impl Choice {
    pub fn has_tool_calls(&self) -> bool {
        self.message.tool_calls.is_some()
    }
}

/// Streaming chunk from OpenRouter
#[derive(Debug, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
pub struct StreamChoice {
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    pub role: Option<String>,
    pub content: Option<String>,
}
