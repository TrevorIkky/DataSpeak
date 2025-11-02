use crate::error::{AppError, AppResult};
use super::types::{OpenRouterRequest, OpenRouterResponse, OpenRouterMessage, StreamChunk, ResponseFormat, Tool};
use futures::stream::Stream;
use reqwest::Client;
use std::pin::Pin;
use tokio_stream::StreamExt;

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

/// OpenRouter API client
pub struct OpenRouterClient {
    client: Client,
    api_key: String,
}

impl OpenRouterClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    /// Call OpenRouter API with tools (returns full response for tool calls)
    pub async fn chat_with_tools(
        &self,
        model: &str,
        messages: &[crate::ai::agent::Message],
        tools: Vec<Tool>,
        temperature: Option<f32>,
    ) -> AppResult<OpenRouterResponse> {
        let openrouter_messages: Vec<OpenRouterMessage> =
            messages.iter().map(|m| m.into()).collect();

        let request = OpenRouterRequest {
            model: model.to_string(),
            messages: openrouter_messages,
            temperature,
            max_tokens: Some(2000),
            stream: Some(false),
            response_format: None,
            tools: Some(tools),
            // Disable parallel tool calls for SQL - queries should run sequentially
            parallel_tool_calls: Some(false),
        };

        let response = self
            .client
            .post(OPENROUTER_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://dataspeak.app")
            .header("X-Title", "DataSpeak")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::OpenRouterError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::OpenRouterError(format!(
                "API error {}: {}",
                status, error_text
            )));
        }

        let api_response: OpenRouterResponse = response
            .json()
            .await
            .map_err(|e| AppError::OpenRouterError(format!("Parse error: {}", e)))?;

        Ok(api_response)
    }

    /// Call OpenRouter API with response format (for structured outputs)
    pub async fn chat_with_format(
        &self,
        model: &str,
        messages: &[crate::ai::agent::Message],
        temperature: Option<f32>,
        response_format: Option<ResponseFormat>,
        tools: Option<Vec<Tool>>,
    ) -> AppResult<String> {
        let openrouter_messages: Vec<OpenRouterMessage> =
            messages.iter().map(|m| m.into()).collect();

        let request = OpenRouterRequest {
            model: model.to_string(),
            messages: openrouter_messages,
            temperature,
            max_tokens: Some(2000),
            stream: Some(false),
            response_format,
            tools,
            parallel_tool_calls: None,
        };

        let response = self
            .client
            .post(OPENROUTER_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://dataspeak.app")
            .header("X-Title", "DataSpeak")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::OpenRouterError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::OpenRouterError(format!(
                "API error {}: {}",
                status, error_text
            )));
        }

        let api_response: OpenRouterResponse = response
            .json()
            .await
            .map_err(|e| AppError::OpenRouterError(format!("Parse error: {}", e)))?;

        api_response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .ok_or_else(|| AppError::OpenRouterError("No response from API".into()))
    }

    /// Call OpenRouter API with streaming
    pub async fn chat_stream(
        &self,
        model: &str,
        messages: &[crate::ai::agent::Message],
        temperature: Option<f32>,
    ) -> AppResult<Pin<Box<dyn Stream<Item = AppResult<String>> + Send>>> {
        let openrouter_messages: Vec<OpenRouterMessage> =
            messages.iter().map(|m| m.into()).collect();

        let request = OpenRouterRequest {
            model: model.to_string(),
            messages: openrouter_messages,
            temperature,
            max_tokens: Some(2000),
            stream: Some(true),
            response_format: None,
            tools: None,
            parallel_tool_calls: None,
        };

        let response = self
            .client
            .post(OPENROUTER_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://dataspeak.app")
            .header("X-Title", "DataSpeak")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::OpenRouterError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::OpenRouterError(format!(
                "API error {}: {}",
                status, error_text
            )));
        }

        // Convert response body to stream of SSE chunks
        let stream = response.bytes_stream();

        let parsed_stream = stream.map(|chunk_result| {
            match chunk_result {
                Ok(bytes) => {
                    // Parse SSE format: "data: {...}\n\n"
                    let text = String::from_utf8_lossy(&bytes);

                    // Handle multiple SSE events in one chunk
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let json_str = line.strip_prefix("data: ").unwrap_or("");

                            // Skip [DONE] marker
                            if json_str == "[DONE]" {
                                continue;
                            }

                            // Parse JSON
                            if let Ok(chunk) = serde_json::from_str::<StreamChunk>(json_str) {
                                if let Some(choice) = chunk.choices.first() {
                                    if let Some(content) = &choice.delta.content {
                                        return Ok(content.clone());
                                    }
                                }
                            }
                        }
                    }

                    // Return empty string if no content in this chunk
                    Ok(String::new())
                }
                Err(e) => Err(AppError::OpenRouterError(format!("Stream error: {}", e))),
            }
        })
        // Filter out empty strings
        .filter(|result| {
            if let Ok(s) = result {
                !s.is_empty()
            } else {
                true // Keep errors
            }
        });

        Ok(Box::pin(parsed_stream))
    }
}
