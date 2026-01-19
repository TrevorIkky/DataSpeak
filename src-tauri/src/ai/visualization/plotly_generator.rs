use crate::ai::agent::QuestionType;
use crate::db::query::QueryResult;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Generated Plotly visualization data (JSON format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotlyVisualization {
    /// Plotly data traces as JSON
    pub data: Vec<Value>,
    /// Plotly layout configuration as JSON
    pub layout: Value,
    /// Chart title for display
    pub title: String,
    /// Chart type (for UI metadata)
    pub chart_type: String,
}

/// Generate Plotly.js code from query result
pub fn generate_plotly_code(
    data: &QueryResult,
    question_type: &QuestionType,
    question: &str,
) -> AppResult<PlotlyVisualization> {
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
        QuestionType::TemporalChart => generate_temporal_chart(data, &temporal_cols, &numeric_cols, question),
        QuestionType::CategoryChart => generate_category_chart(data, &categorical_cols, &numeric_cols, question),
        QuestionType::Statistic => generate_statistic_chart(data, question),
        _ => {
            // Auto-detect based on data
            if !temporal_cols.is_empty() && !numeric_cols.is_empty() {
                generate_temporal_chart(data, &temporal_cols, &numeric_cols, question)
            } else if !categorical_cols.is_empty() && !numeric_cols.is_empty() {
                generate_category_chart(data, &categorical_cols, &numeric_cols, question)
            } else {
                generate_default_chart(data, question)
            }
        }
    }
}

/// Generate a line chart for temporal data
fn generate_temporal_chart(
    data: &QueryResult,
    temporal_cols: &[String],
    numeric_cols: &[String],
    question: &str,
) -> AppResult<PlotlyVisualization> {
    let x_col = temporal_cols
        .first()
        .ok_or_else(|| AppError::VisualizationError("No temporal column found".into()))?;

    let y_cols = if numeric_cols.is_empty() {
        return Err(AppError::VisualizationError(
            "No numeric columns for temporal chart".into(),
        ));
    } else {
        numeric_cols
    };

    // Extract data for each trace
    let x_values = extract_column_values_json(data, x_col);

    let mut traces = Vec::new();
    for y_col in y_cols {
        let y_values = extract_column_values_json(data, y_col);
        traces.push(serde_json::json!({
            "x": x_values,
            "y": y_values,
            "type": "scatter",
            "mode": "lines+markers",
            "name": y_col,
            "line": { "shape": "spline", "smoothing": 0.6 },
            "marker": { "size": 6 }
        }));
    }

    let title = generate_title_from_question(question, "Trend Over Time");
    let layout = serde_json::json!({
        "title": { "text": title, "font": { "size": 16 } },
        "xaxis": {
            "title": x_col,
            "tickangle": -45,
            "automargin": true
        },
        "yaxis": { "title": if y_cols.len() == 1 { &y_cols[0] } else { "Value" } },
        "margin": { "l": 60, "r": 30, "t": 50, "b": 80 },
        "showlegend": y_cols.len() > 1,
        "paper_bgcolor": "transparent",
        "plot_bgcolor": "transparent",
        "font": { "color": "currentColor" }
    });

    Ok(PlotlyVisualization {
        data: traces,
        layout,
        title,
        chart_type: "line".to_string(),
    })
}

/// Generate a bar chart for categorical data
fn generate_category_chart(
    data: &QueryResult,
    categorical_cols: &[String],
    numeric_cols: &[String],
    question: &str,
) -> AppResult<PlotlyVisualization> {
    let x_col = categorical_cols
        .first()
        .ok_or_else(|| AppError::VisualizationError("No categorical column found".into()))?;

    let y_col = numeric_cols
        .first()
        .ok_or_else(|| AppError::VisualizationError("No numeric column found".into()))?;

    let x_values = extract_column_values_json(data, x_col);
    let y_values = extract_column_values_json(data, y_col);

    // Choose chart type based on data characteristics
    let (chart_type, trace) = if data.row_count <= 6 {
        // Pie chart for small datasets
        ("pie", serde_json::json!({
            "labels": x_values,
            "values": y_values,
            "type": "pie",
            "hole": 0.4,
            "textinfo": "label+percent",
            "textposition": "outside",
            "marker": {
                "colors": ["#8884d8", "#82ca9d", "#ffc658", "#ff7300", "#00C49F", "#FFBB28"]
            }
        }))
    } else {
        // Bar chart for larger datasets
        ("bar", serde_json::json!({
            "x": x_values,
            "y": y_values,
            "type": "bar",
            "marker": {
                "color": "#8884d8",
                "line": { "color": "#7773c7", "width": 1 }
            },
            "text": y_values,
            "textposition": "auto"
        }))
    };

    let title = generate_title_from_question(question, "Distribution by Category");
    let layout = if chart_type == "pie" {
        serde_json::json!({
            "title": { "text": title, "font": { "size": 16 } },
            "showlegend": true,
            "legend": { "orientation": "h", "y": -0.1 },
            "margin": { "l": 30, "r": 30, "t": 50, "b": 30 },
            "paper_bgcolor": "transparent",
            "plot_bgcolor": "transparent",
            "font": { "color": "currentColor" }
        })
    } else {
        serde_json::json!({
            "title": { "text": title, "font": { "size": 16 } },
            "xaxis": {
                "title": x_col,
                "tickangle": -45,
                "automargin": true
            },
            "yaxis": { "title": y_col },
            "margin": { "l": 60, "r": 30, "t": 50, "b": 100 },
            "paper_bgcolor": "transparent",
            "plot_bgcolor": "transparent",
            "font": { "color": "currentColor" },
            "bargap": 0.3
        })
    };

    Ok(PlotlyVisualization {
        data: vec![trace],
        layout,
        title,
        chart_type: chart_type.to_string(),
    })
}

/// Generate a statistic indicator chart
fn generate_statistic_chart(data: &QueryResult, question: &str) -> AppResult<PlotlyVisualization> {
    if data.row_count != 1 || data.columns.is_empty() {
        return generate_default_chart(data, question);
    }

    let col = &data.columns[0];
    let value = data
        .rows
        .first()
        .and_then(|row| row.get(col))
        .ok_or_else(|| AppError::VisualizationError("No value found".into()))?;

    let title = generate_title_from_question(question, col);
    let value_format = if is_likely_currency(col, value) { "$,.2f" } else if is_likely_percentage(col) { ".1%" } else { ",.0f" };

    let trace = serde_json::json!({
        "type": "indicator",
        "mode": "number",
        "value": value_to_number_json(value),
        "title": {
            "text": title,
            "font": { "size": 14 }
        },
        "number": {
            "font": { "size": 48, "color": "#8884d8" },
            "valueformat": value_format
        },
        "domain": { "x": [0, 1], "y": [0, 1] }
    });

    let layout = serde_json::json!({
        "margin": { "l": 30, "r": 30, "t": 50, "b": 30 },
        "paper_bgcolor": "transparent",
        "plot_bgcolor": "transparent",
        "font": { "color": "currentColor" }
    });

    Ok(PlotlyVisualization {
        data: vec![trace],
        layout,
        title,
        chart_type: "indicator".to_string(),
    })
}

/// Generate a default bar chart
fn generate_default_chart(data: &QueryResult, question: &str) -> AppResult<PlotlyVisualization> {
    if data.columns.len() < 2 {
        return Err(AppError::VisualizationError(
            "Need at least 2 columns for chart".into(),
        ));
    }

    let x_col = &data.columns[0];
    let y_col = &data.columns[1];

    let x_values = extract_column_values_json(data, x_col);
    let y_values = extract_column_values_json(data, y_col);

    let title = generate_title_from_question(question, "Query Results");

    let trace = serde_json::json!({
        "x": x_values,
        "y": y_values,
        "type": "bar",
        "marker": {
            "color": "#8884d8",
            "line": { "color": "#7773c7", "width": 1 }
        }
    });

    let layout = serde_json::json!({
        "title": { "text": title, "font": { "size": 16 } },
        "xaxis": {
            "title": x_col,
            "tickangle": -45,
            "automargin": true
        },
        "yaxis": { "title": y_col },
        "margin": { "l": 60, "r": 30, "t": 50, "b": 100 },
        "paper_bgcolor": "transparent",
        "plot_bgcolor": "transparent",
        "font": { "color": "currentColor" },
        "bargap": 0.3
    });

    Ok(PlotlyVisualization {
        data: vec![trace],
        layout,
        title,
        chart_type: "bar".to_string(),
    })
}

/// Extract column values as a JSON array
fn extract_column_values_json(data: &QueryResult, column: &str) -> Vec<Value> {
    data.rows
        .iter()
        .map(|row| {
            row.get(column)
                .cloned()
                .unwrap_or(Value::Null)
        })
        .collect()
}

/// Convert a JSON value to a number for Plotly indicators
fn value_to_number_json(value: &Value) -> Value {
    match value {
        Value::Number(_) => value.clone(),
        Value::String(s) => s.parse::<f64>()
            .map(|n| serde_json::json!(n))
            .unwrap_or(serde_json::json!(0)),
        _ => serde_json::json!(0),
    }
}

/// Generate a title from the question or use default
fn generate_title_from_question(question: &str, default: &str) -> String {
    // Extract a meaningful title from the question
    let question_lower = question.to_lowercase();

    if question_lower.starts_with("show ") || question_lower.starts_with("get ") ||
       question_lower.starts_with("list ") || question_lower.starts_with("find ") {
        // Use the rest of the question as title
        let title = question.chars().skip(question.find(' ').unwrap_or(0) + 1).collect::<String>();
        if title.len() > 50 {
            format!("{}...", &title[..47])
        } else if title.is_empty() {
            default.to_string()
        } else {
            capitalize_first(&title)
        }
    } else if question.len() > 50 {
        format!("{}...", &question[..47])
    } else if question.is_empty() {
        default.to_string()
    } else {
        capitalize_first(question)
    }
}

/// Capitalize the first letter
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}

/// Check if a column likely represents currency
fn is_likely_currency(column: &str, _value: &Value) -> bool {
    let col_lower = column.to_lowercase();
    // Only match explicit currency-related column names
    // Avoid "total" as it's too generic (could be count, not money)
    col_lower.contains("price") ||
    col_lower.contains("revenue") ||
    col_lower.contains("cost") ||
    col_lower.contains("sales") ||
    col_lower.contains("balance") ||
    col_lower.contains("fee") ||
    col_lower.contains("payment") ||
    // Only match "amount" if it's combined with money context
    (col_lower.contains("amount") && (
        col_lower.contains("dollar") ||
        col_lower.contains("usd") ||
        col_lower.contains("payment") ||
        col_lower.contains("order")
    )) ||
    // Match "total" only if combined with money context
    (col_lower.contains("total") && (
        col_lower.contains("price") ||
        col_lower.contains("cost") ||
        col_lower.contains("revenue") ||
        col_lower.contains("sales") ||
        col_lower.contains("payment")
    ))
}

/// Check if a column likely represents a percentage
fn is_likely_percentage(column: &str) -> bool {
    let col_lower = column.to_lowercase();
    col_lower.contains("percent") ||
    col_lower.contains("rate") ||
    col_lower.contains("ratio") ||
    col_lower.ends_with("_pct")
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
        if col_lower == "id" || col_lower.ends_with("_id") || col_lower.starts_with("id_") {
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
    s.contains('-') && s.len() >= 8 && s.chars().filter(|c| c.is_numeric()).count() >= 4
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_bar_chart() {
        let mut row1 = serde_json::Map::new();
        row1.insert("category".to_string(), json!("A"));
        row1.insert("value".to_string(), json!(100));

        let mut row2 = serde_json::Map::new();
        row2.insert("category".to_string(), json!("B"));
        row2.insert("value".to_string(), json!(200));

        let data = QueryResult {
            columns: vec!["category".to_string(), "value".to_string()],
            column_metadata: vec![],
            rows: vec![row1, row2],
            row_count: 2,
            execution_time_ms: 0,
        };

        let result = generate_plotly_code(&data, &QuestionType::CategoryChart, "Show values by category");
        assert!(result.is_ok());
        let viz = result.unwrap();
        // Should have data and layout as JSON
        assert!(!viz.data.is_empty());
        assert!(viz.layout.is_object());
        assert_eq!(viz.chart_type, "pie"); // 2 rows = pie chart
    }

    #[test]
    fn test_extract_column_values_json() {
        let mut row1 = serde_json::Map::new();
        row1.insert("name".to_string(), json!("Alice"));
        row1.insert("score".to_string(), json!(95));

        let mut row2 = serde_json::Map::new();
        row2.insert("name".to_string(), json!("Bob"));
        row2.insert("score".to_string(), json!(87));

        let data = QueryResult {
            columns: vec!["name".to_string(), "score".to_string()],
            column_metadata: vec![],
            rows: vec![row1, row2],
            row_count: 2,
            execution_time_ms: 0,
        };

        let names = extract_column_values_json(&data, "name");
        assert_eq!(names, vec![json!("Alice"), json!("Bob")]);

        let scores = extract_column_values_json(&data, "score");
        assert_eq!(scores, vec![json!(95), json!(87)]);
    }
}
