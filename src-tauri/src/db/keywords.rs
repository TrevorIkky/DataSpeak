use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, MySql};
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlKeyword {
    pub word: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Fetch SQL keywords from a PostgreSQL database
async fn fetch_postgres_keywords(pool: &Pool<Postgres>) -> Result<Vec<SqlKeyword>, AppError> {
    let result = sqlx::query(
        r#"
        SELECT word, catcode, catdesc
        FROM pg_get_keywords()
        ORDER BY word
        "#
    )
    .fetch_all(pool)
    .await;

    match result {
        Ok(rows) => {
            let keywords = rows
                .into_iter()
                .map(|row| {
                    use sqlx::Row;
                    let word: String = row.try_get("word").unwrap_or_default();
                    let catcode: String = row.try_get("catcode").unwrap_or_default();
                    let catdesc: Option<String> = row.try_get("catdesc").ok();

                    let category = match catcode.as_str() {
                        "R" => "reserved",
                        "C" => "unreserved_column",
                        "T" => "unreserved_type",
                        "U" => "unreserved",
                        _ => "unknown",
                    };

                    SqlKeyword {
                        word,
                        category: category.to_string(),
                        description: catdesc,
                    }
                })
                .collect();
            Ok(keywords)
        }
        Err(_) => {
            // Fallback to static keywords if query fails
            Ok(get_postgres_fallback_keywords())
        }
    }
}

/// Fetch SQL keywords from a MySQL database (8.0+)
async fn fetch_mysql_keywords(pool: &Pool<MySql>) -> Result<Vec<SqlKeyword>, AppError> {
    // Try MySQL 8.0+ INFORMATION_SCHEMA.KEYWORDS table
    let result = sqlx::query(
        r#"
        SELECT WORD as word, RESERVED as reserved
        FROM INFORMATION_SCHEMA.KEYWORDS
        ORDER BY WORD
        "#
    )
    .fetch_all(pool)
    .await;

    match result {
        Ok(rows) => {
            use sqlx::Row;
            let keywords = rows
                .into_iter()
                .map(|row| {
                    let word: String = row.try_get("word").unwrap_or_default();
                    let reserved: i32 = row.try_get("reserved").unwrap_or(0);

                    SqlKeyword {
                        word,
                        category: if reserved == 1 { "reserved" } else { "unreserved" }.to_string(),
                        description: None,
                    }
                })
                .collect();
            Ok(keywords)
        }
        Err(_) => {
            // Fallback to essential MySQL/MariaDB keywords for older versions
            Ok(get_mysql_fallback_keywords())
        }
    }
}

/// Fallback keywords for MySQL/MariaDB when INFORMATION_SCHEMA.KEYWORDS is unavailable
fn get_mysql_fallback_keywords() -> Vec<SqlKeyword> {
    let keywords = vec![
        // Core SQL keywords
        "SELECT", "FROM", "WHERE", "JOIN", "INNER", "LEFT", "RIGHT", "OUTER", "CROSS",
        "ON", "AND", "OR", "NOT", "IN", "LIKE", "BETWEEN", "IS", "NULL",
        "ORDER", "BY", "GROUP", "HAVING", "LIMIT", "OFFSET",
        "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "TRUNCATE",
        "CREATE", "ALTER", "DROP", "TABLE", "INDEX", "VIEW", "DATABASE", "SCHEMA",
        "AS", "DISTINCT", "ALL", "EXISTS", "CASE", "WHEN", "THEN", "ELSE", "END",
        // Aggregate functions
        "COUNT", "SUM", "AVG", "MIN", "MAX",
        // MySQL-specific
        "AUTO_INCREMENT", "UNSIGNED", "ZEROFILL", "TINYINT", "SMALLINT", "MEDIUMINT",
        "BIGINT", "DECIMAL", "FLOAT", "DOUBLE", "REAL", "BIT", "BOOLEAN", "SERIAL",
        "DATE", "DATETIME", "TIMESTAMP", "TIME", "YEAR",
        "CHAR", "VARCHAR", "BINARY", "VARBINARY", "TINYBLOB", "BLOB", "MEDIUMBLOB", "LONGBLOB",
        "TINYTEXT", "TEXT", "MEDIUMTEXT", "LONGTEXT",
        "ENUM", "SET",
        // String functions
        "CONCAT", "SUBSTRING", "LENGTH", "UPPER", "LOWER", "TRIM", "REPLACE",
        // Date functions
        "NOW", "CURDATE", "CURTIME", "DATE_FORMAT", "DATE_ADD", "DATE_SUB",
        "DATEDIFF", "TIMESTAMPDIFF",
        // Other functions
        "COALESCE", "IFNULL", "NULLIF", "CAST", "CONVERT",
        // Control flow
        "IF", "IFNULL", "NULLIF",
    ];

    keywords
        .into_iter()
        .map(|word| SqlKeyword {
            word: word.to_string(),
            category: "common".to_string(),
            description: None,
        })
        .collect()
}

/// Fetch SQL keywords from PostgreSQL fallback list
fn get_postgres_fallback_keywords() -> Vec<SqlKeyword> {
    let keywords = vec![
        // Core SQL keywords (same as MySQL)
        "SELECT", "FROM", "WHERE", "JOIN", "INNER", "LEFT", "RIGHT", "OUTER", "CROSS",
        "ON", "AND", "OR", "NOT", "IN", "LIKE", "ILIKE", "BETWEEN", "IS", "NULL",
        "ORDER", "BY", "GROUP", "HAVING", "LIMIT", "OFFSET",
        "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "TRUNCATE", "RETURNING",
        "CREATE", "ALTER", "DROP", "TABLE", "INDEX", "VIEW", "DATABASE", "SCHEMA",
        "AS", "DISTINCT", "ALL", "EXISTS", "CASE", "WHEN", "THEN", "ELSE", "END",
        // Aggregate functions
        "COUNT", "SUM", "AVG", "MIN", "MAX", "STRING_AGG", "ARRAY_AGG",
        // PostgreSQL-specific
        "SERIAL", "BIGSERIAL", "SMALLSERIAL",
        "INTEGER", "BIGINT", "SMALLINT", "NUMERIC", "DECIMAL", "REAL", "DOUBLE PRECISION",
        "BOOLEAN", "BYTEA",
        "DATE", "TIMESTAMP", "TIMESTAMPTZ", "TIME", "TIMETZ", "INTERVAL",
        "CHAR", "VARCHAR", "TEXT",
        "JSON", "JSONB", "UUID", "ARRAY",
        "INET", "CIDR", "MACADDR",
        // String functions
        "CONCAT", "SUBSTRING", "LENGTH", "UPPER", "LOWER", "TRIM", "REPLACE",
        "POSITION", "OVERLAY", "SPLIT_PART",
        // Date functions
        "NOW", "CURRENT_DATE", "CURRENT_TIME", "CURRENT_TIMESTAMP",
        "DATE_TRUNC", "EXTRACT", "AGE",
        // JSON functions
        "JSON_BUILD_OBJECT", "JSON_AGG", "JSONB_BUILD_OBJECT", "JSONB_AGG",
        // Other functions
        "COALESCE", "NULLIF", "CAST", "ROW_NUMBER", "RANK", "DENSE_RANK",
    ];

    keywords
        .into_iter()
        .map(|word| SqlKeyword {
            word: word.to_string(),
            category: "common".to_string(),
            description: None,
        })
        .collect()
}

/// Main entry point for fetching SQL keywords
pub async fn fetch_keywords_from_pool(
    manager: &crate::db::connection::ConnectionManager,
    connection_id: &str,
) -> Result<Vec<SqlKeyword>, AppError> {
    use crate::db::connection::DatabaseType;

    let conn = manager.get_connection(connection_id)?;

    match conn.database_type {
        DatabaseType::PostgreSQL => {
            let pool = manager.get_pool_postgres(connection_id).await?;
            fetch_postgres_keywords(&pool).await
        }
        DatabaseType::MariaDB | DatabaseType::MySQL => {
            let pool = manager.get_pool_mysql(connection_id).await?;
            fetch_mysql_keywords(&pool).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mysql_fallback_keywords_not_empty() {
        let keywords = get_mysql_fallback_keywords();
        assert!(!keywords.is_empty());
        assert!(keywords.iter().any(|k| k.word == "SELECT"));
    }

    #[test]
    fn test_postgres_fallback_keywords_not_empty() {
        let keywords = get_postgres_fallback_keywords();
        assert!(!keywords.is_empty());
        assert!(keywords.iter().any(|k| k.word == "SELECT"));
        assert!(keywords.iter().any(|k| k.word == "ILIKE"));
    }
}
