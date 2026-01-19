use crate::ai::agent::{Message, QuestionType};
use crate::ai::openrouter::OpenRouterClient;
use crate::ai::prompts;
use crate::error::AppResult;

/// Classify question using LLM with structured outputs
pub async fn classify_question(
    question: &str,
    openrouter_client: &OpenRouterClient,
    model: &str,
) -> AppResult<QuestionType> {
    use crate::ai::openrouter::types::{ResponseFormat, JsonSchema};

    let classification_prompt = prompts::build_classification_prompt();

    let messages = vec![
        Message::system(classification_prompt.to_string()),
        Message::user(format!("Classify this question: \"{}\"", question)),
    ];

    // Define JSON schema for structured output
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "category": {
                "type": "string",
                "enum": ["general", "table_view", "temporal_chart", "category_chart", "statistic", "complex"],
                "description": "The classification category for the question"
            },
            "confidence": {
                "type": "string",
                "enum": ["high", "medium", "low"],
                "description": "Confidence level in the classification"
            }
        },
        "required": ["category", "confidence"],
        "additionalProperties": false
    });

    let response_format = ResponseFormat {
        format_type: "json_schema".to_string(),
        json_schema: Some(JsonSchema {
            name: "question_classification".to_string(),
            strict: true,
            schema,
        }),
    };

    let response = openrouter_client
        .chat_with_format(model, &messages, Some(0.0), Some(response_format), None)
        .await?;

    // Parse JSON response
    let parsed: serde_json::Value = serde_json::from_str(&response)
        .map_err(|e| crate::error::AppError::Other(format!("Failed to parse classification: {}", e)))?;

    let category = parsed["category"]
        .as_str()
        .ok_or_else(|| crate::error::AppError::Other("Missing category in response".to_string()))?;

    match category {
        "general" => Ok(QuestionType::General),
        "table_view" => Ok(QuestionType::TableView),
        "temporal_chart" => Ok(QuestionType::TemporalChart),
        "category_chart" => Ok(QuestionType::CategoryChart),
        "statistic" => Ok(QuestionType::Statistic),
        "complex" => Ok(QuestionType::Complex),
        _ => Ok(QuestionType::Complex),
    }
}
