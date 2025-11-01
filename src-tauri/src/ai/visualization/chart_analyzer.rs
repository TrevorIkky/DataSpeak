use crate::ai::agent::QuestionType;
use crate::db::query::QueryResult;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Supported chart types with their use cases:
/// - Bar: Comparing discrete categories (e.g., sales by region, counts by status)
/// - Line: Showing trends over time (e.g., user growth, sales over months)
/// - Area: Similar to line but emphasizing volume/magnitude (e.g., cumulative totals)
/// - Pie: Part-to-whole relationships, best for <7 categories (e.g., market share)
/// - Scatter: Correlation between two numeric variables (e.g., price vs quantity)
/// - Radar: Comparing multiple variables across items (e.g., product feature scores)
/// - Radial: Circular progress or part-to-whole with radial display (e.g., completion rates)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChartType {
    Bar,
    Line,
    Area,
    Pie,
    Scatter,
    Radar,
    Radial,
}

/// Visualization configuration matching frontend types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationConfig {
    #[serde(rename = "type")]
    pub chart_type: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub config: ChartConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insights: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartConfig {
    pub x_axis: String,
    pub y_axis: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

/// Generate chart configuration from query result
pub fn generate_config(
    data: &QueryResult,
    question_type: &QuestionType,
) -> AppResult<VisualizationConfig> {
    if data.row_count == 0 {
        return Err(AppError::VisualizationError(
            "Cannot generate chart from empty result set".into(),
        ));
    }

    if data.columns.is_empty() {
        return Err(AppError::VisualizationError("No columns in result".into()));
    }

    // Analyze column types
    let temporal_cols = detect_temporal_columns(&data.columns, &data.rows);
    let numeric_cols = detect_numeric_columns(&data.columns, &data.rows);
    let categorical_cols: Vec<String> = data
        .columns
        .iter()
        .filter(|c| !temporal_cols.contains(c) && !numeric_cols.contains(c))
        .cloned()
        .collect();

    match question_type {
        QuestionType::TemporalChart => generate_temporal_chart_config(
            &data.columns,
            &temporal_cols,
            &numeric_cols,
            data.row_count,
        ),
        QuestionType::CategoryChart => generate_category_chart_config(
            &data.columns,
            &categorical_cols,
            &numeric_cols,
            data.row_count,
        ),
        _ => {
            // Auto-detect based on data
            if !temporal_cols.is_empty() && !numeric_cols.is_empty() {
                generate_temporal_chart_config(
                    &data.columns,
                    &temporal_cols,
                    &numeric_cols,
                    data.row_count,
                )
            } else if !categorical_cols.is_empty() && !numeric_cols.is_empty() {
                generate_category_chart_config(
                    &data.columns,
                    &categorical_cols,
                    &numeric_cols,
                    data.row_count,
                )
            } else {
                // Default to bar chart
                Ok(VisualizationConfig {
                    chart_type: "bar".to_string(),
                    title: "Query Results".to_string(),
                    description: None,
                    config: ChartConfig {
                        x_axis: data.columns.first().cloned().unwrap_or_default(),
                        y_axis: data
                            .columns
                            .get(1)
                            .cloned()
                            .map(|c| vec![c])
                            .unwrap_or_default(),
                        category: None,
                    },
                    insights: None,
                })
            }
        }
    }
}

fn generate_temporal_chart_config(
    _all_columns: &[String],
    temporal_cols: &[String],
    numeric_cols: &[String],
    row_count: usize,
) -> AppResult<VisualizationConfig> {
    let x_axis = temporal_cols
        .first()
        .ok_or_else(|| AppError::VisualizationError("No temporal column found".into()))?
        .clone();

    let y_axis = if numeric_cols.is_empty() {
        // If no numeric columns, count rows
        vec!["count".to_string()]
    } else {
        numeric_cols.to_vec()
    };

    Ok(VisualizationConfig {
        chart_type: "line".to_string(),
        title: "Trend Over Time".to_string(),
        description: Some(format!("Showing {} data points", row_count)),
        config: ChartConfig {
            x_axis,
            y_axis,
            category: None,
        },
        insights: None,
    })
}

fn generate_category_chart_config(
    _all_columns: &[String],
    categorical_cols: &[String],
    numeric_cols: &[String],
    row_count: usize,
) -> AppResult<VisualizationConfig> {
    let x_axis = categorical_cols
        .first()
        .ok_or_else(|| AppError::VisualizationError("No categorical column found".into()))?
        .clone();

    let y_axis = if numeric_cols.is_empty() {
        vec!["count".to_string()]
    } else if numeric_cols.len() > 2 {
        // Multiple metrics - radar chart is good for comparison
        numeric_cols.to_vec()
    } else {
        vec![numeric_cols.first().unwrap().clone()]
    };

    // Choose chart type based on data characteristics
    let chart_type = if numeric_cols.len() > 2 && row_count <= 10 {
        // Multiple metrics across categories - radar chart
        "radar"
    } else if row_count <= 6 && numeric_cols.len() == 1 {
        // Few categories, single metric - pie chart for part-to-whole
        "pie"
    } else if row_count <= 6 && !numeric_cols.is_empty() {
        // Few categories with metrics - radial chart
        "radial"
    } else {
        // Default to bar chart for category comparison
        "bar"
    };

    Ok(VisualizationConfig {
        chart_type: chart_type.to_string(),
        title: "Distribution by Category".to_string(),
        description: Some(format!("Showing {} categories", row_count)),
        config: ChartConfig {
            x_axis,
            y_axis,
            category: categorical_cols.get(1).cloned(),
        },
        insights: None,
    })
}

/// Detect temporal/date columns
fn detect_temporal_columns(columns: &[String], rows: &[serde_json::Map<String, Value>]) -> Vec<String> {
    let mut temporal = Vec::new();

    for col in columns {
        let col_lower = col.to_lowercase();

        // Check column name
        if col_lower.contains("date")
            || col_lower.contains("time")
            || col_lower.contains("created")
            || col_lower.contains("updated")
            || col_lower.contains("timestamp")
            || col_lower == "year"
            || col_lower == "month"
            || col_lower == "day"
        {
            temporal.push(col.clone());
            continue;
        }

        // Check data type by sampling first row
        if let Some(first_row) = rows.first() {
            if let Some(value) = first_row.get(col) {
                if let Some(s) = value.as_str() {
                    // Try to detect date-like strings
                    if is_date_like(s) {
                        temporal.push(col.clone());
                    }
                }
            }
        }
    }

    temporal
}

/// Detect numeric columns
fn detect_numeric_columns(
    columns: &[String],
    rows: &[serde_json::Map<String, Value>],
) -> Vec<String> {
    let mut numeric = Vec::new();

    for col in columns {
        // Skip if it's clearly an ID column
        let col_lower = col.to_lowercase();
        if col_lower == "id"
            || col_lower.ends_with("_id")
            || col_lower.starts_with("id_")
        {
            continue;
        }

        // Check data type by sampling first row
        if let Some(first_row) = rows.first() {
            if let Some(value) = first_row.get(col) {
                if value.is_number() {
                    numeric.push(col.clone());
                }
            }
        }
    }

    numeric
}

/// Simple date string detection
fn is_date_like(s: &str) -> bool {
    // Check for common date formats
    s.contains('-') && (s.len() >= 8) && s.chars().filter(|c| c.is_numeric()).count() >= 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_temporal_by_name() {
        let columns = vec![
            "id".to_string(),
            "created_at".to_string(),
            "name".to_string(),
        ];
        let rows = vec![];

        let temporal = detect_temporal_columns(&columns, &rows);
        assert!(temporal.contains(&"created_at".to_string()));
    }

    #[test]
    fn test_detect_numeric() {
        let columns = vec!["id".to_string(), "name".to_string(), "age".to_string()];

        let mut row = serde_json::Map::new();
        row.insert("id".to_string(), Value::Number(1.into()));
        row.insert("name".to_string(), Value::String("John".to_string()));
        row.insert("age".to_string(), Value::Number(30.into()));

        let rows = vec![row];

        let numeric = detect_numeric_columns(&columns, &rows);
        // Should include age but not id (filtered out)
        assert!(numeric.contains(&"age".to_string()));
        assert!(!numeric.contains(&"id".to_string()));
    }
}
