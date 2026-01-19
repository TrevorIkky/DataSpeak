use crate::error::{AppError, AppResult};
use super::types::{OpenRouterRequest, OpenRouterResponse, OpenRouterMessage, ResponseFormat, Tool};
use reqwest::Client;

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
}
