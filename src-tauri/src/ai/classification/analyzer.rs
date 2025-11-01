use crate::ai::agent::{Message, QuestionType};
use crate::ai::openrouter::OpenRouterClient;
use crate::ai::prompts;
use crate::error::AppResult;
use regex::Regex;
use std::sync::LazyLock;

/// Heuristic patterns for fast classification
static TABLE_VIEW_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(show|display|list|view|get|fetch|retrieve|see)\b.*\b(users?|products?|orders?|customers?|items?|records?|rows?|data|table)\b").unwrap()
});

static STATISTIC_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(how many|count|total|sum|average|mean|number of)\b").unwrap()
});

static TEMPORAL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(over time|trend|timeline|last \d+|past \d+|since|between|during|growth|historical)\b").unwrap()
});

static CATEGORY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bby\s+(country|category|type|status|region|state|city|group)\b").unwrap()
});

static VISUALIZATION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(visualiz|visualis|graph|chart|plot|bar\s+graph|bar\s+chart|pie\s+chart|line\s+graph|line\s+chart)\b").unwrap()
});

/// Classify question using fast heuristics first, then LLM if needed
pub async fn classify_question(
    question: &str,
    openrouter_client: &OpenRouterClient,
    model: &str,
    use_llm_fallback: bool,
) -> AppResult<QuestionType> {
    // Try heuristic classification first (fast path)
    if let Some(question_type) = classify_heuristic(question) {
        return Ok(question_type);
    }

    // Fall back to LLM classification if enabled
    if use_llm_fallback {
        classify_with_llm(question, openrouter_client, model).await
    } else {
        // Default to Complex if no heuristic match and LLM disabled
        Ok(QuestionType::Complex)
    }
}

/// Fast heuristic-based classification
fn classify_heuristic(question: &str) -> Option<QuestionType> {
    let question_lower = question.to_lowercase();

    // Check for explicit visualization request
    let has_viz_request = VISUALIZATION_PATTERN.is_match(&question_lower);

    // Check for table view pattern (but not if explicitly asking for visualization)
    if TABLE_VIEW_PATTERN.is_match(&question_lower) && !has_viz_request {
        // Make sure it's not also asking for temporal data
        if !TEMPORAL_PATTERN.is_match(&question_lower) {
            return Some(QuestionType::TableView);
        }
    }

    // Check for statistic (but not if it's also temporal/trending or asking for visualization)
    if STATISTIC_PATTERN.is_match(&question_lower) && !has_viz_request {
        if !TEMPORAL_PATTERN.is_match(&question_lower)
            && !CATEGORY_PATTERN.is_match(&question_lower)
        {
            return Some(QuestionType::Statistic);
        }
    }

    // Check for temporal chart
    if TEMPORAL_PATTERN.is_match(&question_lower) {
        return Some(QuestionType::TemporalChart);
    }

    // Check for category chart or explicit visualization request
    if CATEGORY_PATTERN.is_match(&question_lower) || has_viz_request {
        return Some(QuestionType::CategoryChart);
    }

    // No clear match
    None
}

/// LLM-based classification for ambiguous cases using structured outputs
async fn classify_with_llm(
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
        _ => Ok(QuestionType::Complex), // Default fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heuristic_table_view() {
        let result = classify_heuristic("show me all users");
        assert_eq!(result, Some(QuestionType::TableView));

        let result = classify_heuristic("list products");
        assert_eq!(result, Some(QuestionType::TableView));

        let result = classify_heuristic("display orders");
        assert_eq!(result, Some(QuestionType::TableView));
    }

    #[test]
    fn test_heuristic_statistic() {
        let result = classify_heuristic("how many users are there");
        assert_eq!(result, Some(QuestionType::Statistic));

        let result = classify_heuristic("count total orders");
        assert_eq!(result, Some(QuestionType::Statistic));

        let result = classify_heuristic("what is the average order value");
        assert_eq!(result, Some(QuestionType::Statistic));
    }

    #[test]
    fn test_heuristic_temporal() {
        let result = classify_heuristic("users joined in the last 7 days");
        assert_eq!(result, Some(QuestionType::TemporalChart));

        let result = classify_heuristic("sales trend over time");
        assert_eq!(result, Some(QuestionType::TemporalChart));

        let result = classify_heuristic("growth since January");
        assert_eq!(result, Some(QuestionType::TemporalChart));
    }

    #[test]
    fn test_heuristic_category() {
        let result = classify_heuristic("users by country");
        assert_eq!(result, Some(QuestionType::CategoryChart));

        let result = classify_heuristic("products by category");
        assert_eq!(result, Some(QuestionType::CategoryChart));

        let result = classify_heuristic("orders by status");
        assert_eq!(result, Some(QuestionType::CategoryChart));
    }

    #[test]
    fn test_temporal_overrides_statistic() {
        // "how many" triggers statistic, but "last 7 days" makes it temporal
        let result = classify_heuristic("how many users joined in the last 7 days");
        assert_eq!(result, Some(QuestionType::TemporalChart));
    }
}
