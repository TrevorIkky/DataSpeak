use crate::error::{AppError, AppResult};
use super::types::{OpenRouterRequest, OpenRouterResponse, OpenRouterMessage, StreamChunk, ResponseFormat, Tool, StreamEvent, ToolCall, FunctionCall};
use futures::stream::Stream;
use futures::StreamExt;
use reqwest::Client;
use std::pin::Pin;
use std::collections::HashMap;
use tokio_util::bytes::Bytes;

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
            futures::future::ready(match result {
                Ok(s) if s.is_empty() => false,
                _ => true,
            })
        });

        Ok(Box::pin(parsed_stream))
    }

    /// Call OpenRouter API with tools and streaming
    pub async fn chat_with_tools_stream(
        &self,
        model: &str,
        messages: &[crate::ai::agent::Message],
        tools: Vec<Tool>,
        temperature: Option<f32>,
    ) -> AppResult<Pin<Box<dyn Stream<Item = AppResult<StreamEvent>> + Send>>> {
        let openrouter_messages: Vec<OpenRouterMessage> =
            messages.iter().map(|m| m.into()).collect();

        let request = OpenRouterRequest {
            model: model.to_string(),
            messages: openrouter_messages,
            temperature,
            max_tokens: Some(2000),
            stream: Some(true),
            response_format: None,
            tools: Some(tools),
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

        // Convert response body to stream of SSE chunks
        let stream = response.bytes_stream();

        // Use stateful stream processing to maintain tool call accumulation across chunks
        use std::sync::{Arc, Mutex};
        let tool_calls_map = Arc::new(Mutex::new(HashMap::<usize, ToolCall>::new()));

        let parsed_stream = stream.flat_map({
            let tool_calls_map = Arc::clone(&tool_calls_map);
            move |chunk_result: Result<Bytes, _>| {
                let events: Vec<AppResult<StreamEvent>> = match chunk_result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        let mut result_events = Vec::new();

                        // Handle multiple SSE events in one chunk
                        for line in text.lines() {
                            if line.starts_with("data: ") {
                                let json_str = line.strip_prefix("data: ").unwrap_or("");

                                // Skip [DONE] marker
                                if json_str == "[DONE]" {
                                    result_events.push(Ok(StreamEvent::Done));
                                    continue;
                                }

                                // Parse JSON
                                if let Ok(chunk) = serde_json::from_str::<StreamChunk>(json_str) {
                                    if let Some(choice) = chunk.choices.first() {
                                        // Handle content
                                        if let Some(content) = &choice.delta.content {
                                            if !content.is_empty() {
                                                result_events.push(Ok(StreamEvent::Content(content.clone())));
                                            }
                                        }

                                        // Handle tool call deltas
                                        if let Some(delta_tool_calls) = &choice.delta.tool_calls {
                                            let mut map = tool_calls_map.lock().unwrap();
                                            for delta_tc in delta_tool_calls {
                                                let entry = map
                                                    .entry(delta_tc.index)
                                                    .or_insert_with(|| ToolCall {
                                                        id: String::new(),
                                                        call_type: String::from("function"),
                                                        function: FunctionCall {
                                                            name: String::new(),
                                                            arguments: String::new(),
                                                        },
                                                    });

                                                // Accumulate tool call data
                                                if let Some(id) = &delta_tc.id {
                                                    entry.id = id.clone();
                                                }
                                                if let Some(call_type) = &delta_tc.call_type {
                                                    entry.call_type = call_type.clone();
                                                }
                                                if let Some(func) = &delta_tc.function {
                                                    if let Some(name) = &func.name {
                                                        entry.function.name = name.clone();
                                                    }
                                                    if let Some(args) = &func.arguments {
                                                        entry.function.arguments.push_str(args);
                                                    }
                                                }
                                            }
                                        }

                                        // Check finish reason
                                        if let Some(finish_reason) = &choice.finish_reason {
                                            let mut map = tool_calls_map.lock().unwrap();
                                            if finish_reason == "tool_calls" && !map.is_empty() {
                                                // Collect all accumulated tool calls
                                                let mut complete_tool_calls: Vec<ToolCall> = map
                                                    .values()
                                                    .cloned()
                                                    .collect();
                                                complete_tool_calls.sort_by_key(|tc| {
                                                    // Extract index from id if possible, fallback to 0
                                                    tc.id.split('_').last()
                                                        .and_then(|s| s.parse::<usize>().ok())
                                                        .unwrap_or(0)
                                                });
                                                result_events.push(Ok(StreamEvent::ToolCalls(complete_tool_calls)));
                                                map.clear();
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        result_events
                    }
                    Err(e) => {
                        vec![Err(AppError::OpenRouterError(format!("Stream error: {}", e)))]
                    }
                };

                futures::stream::iter(events)
            }
        })
        // Filter out empty content events
        .filter(|result| {
            futures::future::ready(match result {
                Ok(StreamEvent::Content(s)) if s.is_empty() => false,
                _ => true,
            })
        });

        Ok(Box::pin(parsed_stream))
    }
}
