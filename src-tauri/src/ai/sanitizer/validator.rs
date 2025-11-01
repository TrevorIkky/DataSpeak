use crate::error::{AppError, AppResult};
use regex::Regex;
use std::sync::LazyLock;

/// SQL injection prevention patterns
static DENY_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // DML/DDL keywords
        Regex::new(r"(?i)\b(INSERT|UPDATE|DELETE|DROP|ALTER|CREATE|TRUNCATE|REPLACE|GRANT|REVOKE)\b").unwrap(),
        // Multiple statements
        Regex::new(r";.*;").unwrap(),
        // SQL comments (potential injection vectors)
        Regex::new(r"--").unwrap(),
        Regex::new(r"/\*").unwrap(),
        // Union-based injection
        Regex::new(r"(?i)\bUNION\b.*\bSELECT\b").unwrap(),
        // Stacked queries
        Regex::new(r";\s*(SELECT|INSERT|UPDATE|DELETE|DROP|ALTER|CREATE)").unwrap(),
    ]
});

/// Check if query has LIMIT clause
static HAS_LIMIT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bLIMIT\s+\d+").unwrap()
});

/// Validate and sanitize SQL query for agent execution
pub fn validate_sql(query: &str) -> AppResult<String> {
    let trimmed = query.trim();

    // Must not be empty
    if trimmed.is_empty() {
        return Err(AppError::SecurityError("Empty query".into()));
    }

    let normalized = trimmed.to_uppercase();

    // Must start with SELECT
    if !normalized.starts_with("SELECT") {
        return Err(AppError::SecurityError(
            "Only SELECT queries are allowed for AI agent".into(),
        ));
    }

    // Check all deny patterns
    for (idx, pattern) in DENY_PATTERNS.iter().enumerate() {
        if pattern.is_match(trimmed) {
            return Err(AppError::SecurityError(format!(
                "Forbidden SQL pattern detected (rule {}): {}",
                idx + 1,
                pattern.as_str()
            )));
        }
    }

    // Build sanitized query
    let mut sanitized = trimmed.to_string();

    // Remove trailing semicolons
    while sanitized.ends_with(';') {
        sanitized.pop();
    }

    // Ensure LIMIT exists (max 100 rows for AI)
    if !HAS_LIMIT_RE.is_match(&sanitized) {
        sanitized.push_str(" LIMIT 100");
    } else {
        // Check that LIMIT doesn't exceed 100
        if let Some(captures) = Regex::new(r"(?i)LIMIT\s+(\d+)").unwrap().captures(&sanitized) {
            if let Some(limit_str) = captures.get(1) {
                if let Ok(limit) = limit_str.as_str().parse::<i32>() {
                    if limit > 100 {
                        // Replace with max limit
                        sanitized = Regex::new(r"(?i)LIMIT\s+\d+")
                            .unwrap()
                            .replace(&sanitized, "LIMIT 100")
                            .to_string();
                    }
                }
            }
        }
    }

    Ok(sanitized)
}

/// Additional validation for specific database types
pub fn validate_for_db_type(query: &str, db_type: &str) -> AppResult<()> {
    match db_type {
        "postgres" => {
            // Postgres-specific checks
            // Block pgcrypto or admin functions
            if query.contains("pg_") || query.contains("pgcrypto") {
                return Err(AppError::SecurityError(
                    "PostgreSQL system functions not allowed".into(),
                ));
            }
        }
        "mysql" | "mariadb" => {
            // MySQL/MariaDB-specific checks
            if query.contains("LOAD_FILE") || query.contains("INTO OUTFILE") {
                return Err(AppError::SecurityError(
                    "File operations not allowed".into(),
                ));
            }
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_select() {
        let result = validate_sql("SELECT * FROM users");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "SELECT * FROM users LIMIT 100");
    }

    #[test]
    fn test_select_with_limit() {
        let result = validate_sql("SELECT * FROM users LIMIT 50");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "SELECT * FROM users LIMIT 50");
    }

    #[test]
    fn test_limit_too_high() {
        let result = validate_sql("SELECT * FROM users LIMIT 500");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "SELECT * FROM users LIMIT 100");
    }

    #[test]
    fn test_reject_insert() {
        let result = validate_sql("INSERT INTO users (name) VALUES ('test')");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_update() {
        let result = validate_sql("UPDATE users SET name = 'test'");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_delete() {
        let result = validate_sql("DELETE FROM users");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_drop() {
        let result = validate_sql("DROP TABLE users");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_comment() {
        let result = validate_sql("SELECT * FROM users -- comment");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_union_injection() {
        let result = validate_sql("SELECT * FROM users UNION SELECT * FROM passwords");
        assert!(result.is_err());
    }

    #[test]
    fn test_complex_valid_query() {
        let query = "SELECT u.id, u.name, COUNT(o.id) as order_count
                     FROM users u
                     LEFT JOIN orders o ON u.id = o.user_id
                     WHERE u.created_at > '2024-01-01'
                     GROUP BY u.id, u.name
                     ORDER BY order_count DESC";
        let result = validate_sql(query);
        assert!(result.is_ok());
    }
}
