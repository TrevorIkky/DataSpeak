/// Build a minimal prompt for question classification
pub fn build_classification_prompt() -> &'static str {
    r#"Classify the user's question into ONE of these categories:

1. general: Greetings, pleasantries, or non-data questions
   Examples: "hi", "hello", "how are you", "thanks", "what can you do"

2. table_view: User wants to see/display/list multiple rows from a table
   Examples: "show me users", "display all products", "list orders", "view customers"

3. temporal_chart: User wants to see a TREND or TIME-SERIES with multiple data points over time
   Examples: "users joined over time", "sales trend last 30 days", "growth over months", "daily signups chart"
   NOT for single-value questions like "when did X happen" or "what was the last X"

4. category_chart: User wants to see data grouped by categories (bar chart, pie chart)
   Examples: "users by country", "products by category", "sales by region", "distribution of statuses"

5. statistic: User wants a SINGLE value, count, date, or metric
   Examples: "how many users", "total revenue", "when did the last user log in", "what is the latest order date", "average order value"
   Use this for any question expecting ONE answer (number, date, name, etc.)

6. complex: Multi-step analysis requiring joins or complex aggregation
   Examples: "top 10 customers by lifetime value", "cohort analysis", "users who ordered more than 3 times"

Return the category that best matches."#
}
