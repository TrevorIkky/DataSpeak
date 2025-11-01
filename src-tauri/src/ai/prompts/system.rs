use crate::ai::agent::QuestionType;

/// Build system prompt for the agent based on question type and schema
pub fn build_system_prompt(schema: &str, question_type: &QuestionType) -> String {
    // For general questions, use a simple conversational prompt
    if matches!(question_type, QuestionType::General) {
        return r#"You are a friendly AI assistant for DataSpeak, a database analysis tool.

The user is greeting you or asking a general question that doesn't require database access.
Respond in a warm, helpful, and concise manner.

For greetings: Be friendly and let them know you're ready to help analyze their data.
For questions about your capabilities: Explain that you can help them query and visualize their database data.

RESPONSE FORMAT:
Final Answer: [Your friendly, concise response]

Keep your response brief (1-3 sentences) and conversational."#.to_string();
    }

    let base = format!(
        r#"You are an expert SQL analyst assistant helping users query and analyze their database.

DATABASE SCHEMA:
{}

You have access to the execute_sql tool to run SELECT queries on the database.

INSTRUCTIONS:
1. Analyze the user's question carefully
2. Write a SQL query to get the needed data (SELECT only, max 100 rows with LIMIT)
3. Call execute_sql with your query
4. Once you have the results, provide a clear, concise answer to the user

RULES:
- Only SELECT queries allowed (no INSERT, UPDATE, DELETE, DROP, ALTER, CREATE)
- Always include LIMIT clause (maximum 100 rows)
- Use correct SQL syntax for the database shown in schema
- Keep answers brief and focused on what the user asked"#,
        schema
    );

    // Add type-specific guidance
    match question_type {
        QuestionType::General => unreachable!(), // Already handled above
        QuestionType::TableView => {
            format!("{}\n\nSPECIAL INSTRUCTION: The user wants to view table data. Return rows from the appropriate table with SELECT. Include relevant columns and use LIMIT appropriately.", base)
        }
        QuestionType::TemporalChart => {
            format!("{}\n\nSPECIAL INSTRUCTION: The user wants time-series data for visualization. Your query should:\n- Include a date/time column\n- Aggregate data by time period if needed (day, week, month)\n- Order by date\n- Include count or sum for the metric being tracked\n\nAVAILABLE CHART TYPES:\n- line: Best for trends over time\n- area: Emphasize magnitude/volume over time\n- bar: Discrete time comparisons", base)
        }
        QuestionType::CategoryChart => {
            format!("{}\n\nSPECIAL INSTRUCTION: The user wants categorical data for visualization. Your query should:\n- Group by the category column\n- Include aggregations (COUNT, SUM, AVG) for the metric\n- Order by the metric to show top categories\n\nAVAILABLE CHART TYPES:\n- bar: Best for comparing categories\n- pie/radial: Good for part-to-whole with <7 categories\n- radar: Compare multiple metrics across categories", base)
        }
        QuestionType::Statistic => {
            format!("{}\n\nSPECIAL INSTRUCTION: The user wants a specific statistic or count. Your query should use aggregate functions (COUNT, SUM, AVG, MIN, MAX) to calculate the exact number requested.", base)
        }
        QuestionType::Complex => {
            format!("{}\n\nSPECIAL INSTRUCTION: This is a complex analytical question. Break it down into steps:\n1. Understand what data is needed\n2. Query the necessary information (you may need multiple queries)\n3. Analyze and synthesize the results\n4. Provide a comprehensive answer", base)
        }
    }
}

/// Build a minimal prompt for question classification
pub fn build_classification_prompt() -> &'static str {
    r#"Classify the user's question into ONE of these categories:

1. GENERAL: Greetings, pleasantries, or non-data questions
   Examples: "hi", "hello", "how are you", "thanks", "what can you do"

2. TABLE_VIEW: User wants to see/display/list rows from a table (but NOT visualize)
   Examples: "show me users", "display all products", "list orders"

3. TEMPORAL_CHART: User wants to see data over time (trend, timeline)
   Examples: "users joined over time", "sales last 30 days", "growth trend"

4. CATEGORY_CHART: User wants to see distribution by category OR requests visualization (chart/graph/plot)
   Examples: "users by country", "products by category", "visualize permissions", "bar chart of sales", "plot data"

5. STATISTIC: User wants a single number (count, sum, average) without visualization
   Examples: "how many users", "total revenue", "average order value"

6. COMPLEX: Multi-step analysis or complex aggregation
   Examples: "top 10 customers by lifetime value", "cohort analysis"

Respond with ONLY the category name, nothing else."#
}
