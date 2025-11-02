use crate::ai::agent::QuestionType;

/// Build system prompt for the agent based on question type and schema
pub fn build_system_prompt(schema: &str, question_type: &QuestionType) -> String {
    // For general questions, use a simple conversational prompt
    if matches!(question_type, QuestionType::General) {
        return r#"You are a friendly AI assistant for DataSpeak, a database analysis tool.

The user is greeting you or asking a general question that doesn't require database access.
Respond in a warm, helpful, and concise manner.

For greetings: Be friendly and let them know you're ready to help analyze their data.
For questions about your capabilities: Explain that you can help them query and analyze their database data.

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
2. Determine the best approach to answer their question based on available tools
3. Write a SQL query to get the needed data (SELECT only, max 100 rows with LIMIT)
4. Call execute_sql with your query
5. Once you have the results, provide a clear, concise answer to the user

RULES:
- Only SELECT queries allowed (no INSERT, UPDATE, DELETE, DROP, ALTER, CREATE)
- Always include LIMIT clause (maximum 100 rows)
- Use correct SQL syntax for the database shown in schema
- Keep answers brief and focused on what the user asked
- Let the data guide your response - not all data needs visualization"#,
        schema
    );

    // Add type-specific guidance
    match question_type {
        QuestionType::General => unreachable!(), // Already handled above
        QuestionType::TableView => {
            format!("{}\n\nCONTEXT: The user wants to view table data. Query the appropriate table with SELECT, including relevant columns and using LIMIT appropriately.", base)
        }
        QuestionType::TemporalChart => {
            format!("{}\n\nCONTEXT: The user's question involves time-series or temporal data. Your query should:\n- Include a date/time column if analyzing trends\n- Aggregate data by time period if appropriate (day, week, month)\n- Order by date when relevant\n- Include the metrics being tracked\n\nDecide based on the question whether visualization would be helpful.", base)
        }
        QuestionType::CategoryChart => {
            format!("{}\n\nCONTEXT: The user's question involves categorical or grouped data. Your query should:\n- Group by the category column when appropriate\n- Include aggregations (COUNT, SUM, AVG) if analyzing metrics\n- Order results logically (by metric or category)\n\nDecide based on the question and data whether visualization would be helpful.", base)
        }
        QuestionType::Statistic => {
            format!("{}\n\nCONTEXT: The user wants a specific statistic or count. Your query should use aggregate functions (COUNT, SUM, AVG, MIN, MAX) to calculate the requested value.", base)
        }
        QuestionType::Complex => {
            format!("{}\n\nCONTEXT: This is a complex analytical question. Break it down into steps:\n1. Understand what data is needed\n2. Query the necessary information (you may need multiple queries if needed)\n3. Analyze and synthesize the results\n4. Provide a comprehensive answer based on the data", base)
        }
    }
}

/// Build a minimal prompt for question classification
pub fn build_classification_prompt() -> &'static str {
    r#"Classify the user's question into ONE of these categories:

1. GENERAL: Greetings, pleasantries, or non-data questions
   Examples: "hi", "hello", "how are you", "thanks", "what can you do"

2. TABLE_VIEW: User wants to see/display/list rows from a table
   Examples: "show me users", "display all products", "list orders", "view customers"

3. TEMPORAL_CHART: User's question involves time-series or temporal data
   Examples: "users joined over time", "sales last 30 days", "growth trend", "monthly signups"

4. CATEGORY_CHART: User's question involves categorical grouping or distribution
   Examples: "users by country", "products by category", "sales by region", "distribution of statuses"

5. STATISTIC: User wants a specific number, count, or metric
   Examples: "how many users", "total revenue", "average order value", "sum of sales"

6. COMPLEX: Multi-step analysis or complex aggregation
   Examples: "top 10 customers by lifetime value", "cohort analysis", "retention rate"

Respond with ONLY the category name, nothing else."#
}
