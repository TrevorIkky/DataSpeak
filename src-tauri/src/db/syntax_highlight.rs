use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::db::keywords::SqlKeyword;
use crate::db::schema::Schema;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightConfig {
    pub keywords: Vec<SqlKeyword>,
    pub schema: Option<Schema>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenType {
    KeywordDml,         // SELECT, INSERT, UPDATE, DELETE
    KeywordClause,      // FROM, WHERE, JOIN, ORDER, GROUP
    KeywordReserved,    // Other reserved keywords
    KeywordUnreserved,
    KeywordType,
    KeywordFunction,
    KeywordCommon,
    Table,
    Column,
    String,
    Number,
    Operator,
    Comment,
    Text,
}

struct Token {
    token_type: TokenType,
    value: String,
}

/// Tokenize SQL text with syntax highlighting
pub fn highlight_sql(sql: &str, config: &HighlightConfig) -> String {
    let tokens = tokenize_sql(sql, config);
    tokens_to_html(&tokens)
}

fn tokenize_sql(sql: &str, config: &HighlightConfig) -> Vec<Token> {
    let mut tokens = Vec::new();

    // Build lookup maps
    let keyword_map: HashMap<String, String> = config
        .keywords
        .iter()
        .map(|kw| (kw.word.to_uppercase(), kw.category.clone()))
        .collect();

    let mut table_set = HashSet::new();
    let mut column_set = HashSet::new();

    if let Some(schema) = &config.schema {
        for table in &schema.tables {
            table_set.insert(table.name.to_uppercase());
            for column in &table.columns {
                column_set.insert(column.name.to_uppercase());
            }
        }
    }

    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        // Single-line comment (-- comment)
        if ch == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
            let mut comment = String::new();
            while i < chars.len() && chars[i] != '\n' {
                comment.push(chars[i]);
                i += 1;
            }
            tokens.push(Token {
                token_type: TokenType::Comment,
                value: comment,
            });
            continue;
        }

        // Multi-line comment (/* comment */)
        if ch == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            let mut comment = String::new();
            while i < chars.len() {
                comment.push(chars[i]);
                if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                    comment.push(chars[i + 1]);
                    i += 2;
                    break;
                }
                i += 1;
            }
            tokens.push(Token {
                token_type: TokenType::Comment,
                value: comment,
            });
            continue;
        }

        // String literals (single quotes)
        if ch == '\'' {
            let mut string = String::from("'");
            i += 1;
            while i < chars.len() {
                if chars[i] == '\'' {
                    if i + 1 < chars.len() && chars[i + 1] == '\'' {
                        string.push_str("''");
                        i += 2;
                    } else {
                        string.push('\'');
                        i += 1;
                        break;
                    }
                } else if chars[i] == '\\' && i + 1 < chars.len() {
                    string.push(chars[i]);
                    string.push(chars[i + 1]);
                    i += 2;
                } else {
                    string.push(chars[i]);
                    i += 1;
                }
            }
            tokens.push(Token {
                token_type: TokenType::String,
                value: string,
            });
            continue;
        }

        // Identifiers with double quotes or backticks
        if ch == '"' || ch == '`' {
            let quote = ch;
            let mut string = String::from(quote);
            i += 1;
            while i < chars.len() {
                if chars[i] == quote {
                    string.push(quote);
                    i += 1;
                    break;
                } else {
                    string.push(chars[i]);
                    i += 1;
                }
            }
            tokens.push(Token {
                token_type: TokenType::Text,
                value: string,
            });
            continue;
        }

        // Numbers
        if ch.is_ascii_digit() {
            let mut num = String::new();
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                num.push(chars[i]);
                i += 1;
            }
            tokens.push(Token {
                token_type: TokenType::Number,
                value: num,
            });
            continue;
        }

        // Identifiers and keywords
        if ch.is_alphabetic() || ch == '_' {
            // Try to match alias.column pattern
            if let Some((alias, column, length)) = try_match_aliased_column(&chars, i, &column_set) {
                tokens.push(Token {
                    token_type: TokenType::Text,
                    value: alias,
                });
                tokens.push(Token {
                    token_type: TokenType::Operator,
                    value: ".".to_string(),
                });
                tokens.push(Token {
                    token_type: TokenType::Column,
                    value: column,
                });
                i += length;
                continue;
            }

            // Regular identifier
            let mut word = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                word.push(chars[i]);
                i += 1;
            }

            let upper_word = word.to_uppercase();

            let token_type = if let Some(category) = keyword_map.get(&upper_word) {
                // First check for specific keyword types regardless of category
                match upper_word.as_str() {
                    // DML keywords (Data Manipulation Language)
                    "SELECT" | "INSERT" | "UPDATE" | "DELETE" | "TRUNCATE" => TokenType::KeywordDml,
                    // Clause keywords
                    "FROM" | "WHERE" | "JOIN" | "INNER" | "LEFT" | "RIGHT" | "OUTER" | "CROSS"
                    | "ON" | "AND" | "OR" | "NOT" | "IN" | "EXISTS" | "BETWEEN" | "LIKE" | "IS"
                    | "ORDER" | "BY" | "GROUP" | "HAVING" | "LIMIT" | "OFFSET" | "DISTINCT"
                    | "AS" | "UNION" | "ALL" | "INTERSECT" | "EXCEPT" => TokenType::KeywordClause,
                    _ => {
                        // Fall back to category-based classification
                        match category.as_str() {
                            "reserved" => TokenType::KeywordReserved,
                            "unreserved" | "unreserved_column" => TokenType::KeywordUnreserved,
                            "unreserved_type" => TokenType::KeywordType,
                            "function" => TokenType::KeywordFunction,
                            _ => TokenType::KeywordCommon,
                        }
                    }
                }
            } else if table_set.contains(&upper_word) {
                TokenType::Table
            } else if column_set.contains(&upper_word) {
                TokenType::Column
            } else {
                TokenType::Text
            };

            tokens.push(Token {
                token_type,
                value: word,
            });
            continue;
        }

        // Operators (two-character)
        if i + 1 < chars.len() {
            let two_char = format!("{}{}", ch, chars[i + 1]);
            if matches!(two_char.as_str(), "<>" | "!=" | "<=" | ">=" | "||") {
                tokens.push(Token {
                    token_type: TokenType::Operator,
                    value: two_char,
                });
                i += 2;
                continue;
            }
        }

        // Single-character operators
        if matches!(ch, '=' | '<' | '>' | '+' | '-' | '*' | '/' | '%' | '(' | ')' | ',' | ';' | '.') {
            tokens.push(Token {
                token_type: TokenType::Operator,
                value: ch.to_string(),
            });
            i += 1;
            continue;
        }

        // Everything else (whitespace, punctuation, etc.)
        tokens.push(Token {
            token_type: TokenType::Text,
            value: ch.to_string(),
        });
        i += 1;
    }

    tokens
}

/// Try to match alias.column pattern
fn try_match_aliased_column(
    chars: &[char],
    start_pos: usize,
    column_set: &HashSet<String>,
) -> Option<(String, String, usize)> {
    let mut pos = start_pos;

    // Read first identifier (potential alias)
    if pos >= chars.len() || !(chars[pos].is_alphabetic() || chars[pos] == '_') {
        return None;
    }

    let mut alias = String::new();
    while pos < chars.len() && (chars[pos].is_alphanumeric() || chars[pos] == '_') {
        alias.push(chars[pos]);
        pos += 1;
    }

    // Check for dot
    if pos >= chars.len() || chars[pos] != '.' {
        return None;
    }
    pos += 1; // Skip dot

    // Read second identifier (potential column)
    if pos >= chars.len() || !(chars[pos].is_alphabetic() || chars[pos] == '_') {
        return None;
    }

    let mut column = String::new();
    while pos < chars.len() && (chars[pos].is_alphanumeric() || chars[pos] == '_') {
        column.push(chars[pos]);
        pos += 1;
    }

    // Check if the column exists in our schema
    if column_set.contains(&column.to_uppercase()) {
        Some((alias, column, pos - start_pos))
    } else {
        None
    }
}

fn tokens_to_html(tokens: &[Token]) -> String {
    let mut html = String::new();

    for token in tokens {
        let escaped = escape_html(&token.value);

        let wrapped = match token.token_type {
            TokenType::KeywordDml => format!("<span class=\"sql-keyword-dml\">{}</span>", escaped),
            TokenType::KeywordClause => format!("<span class=\"sql-keyword-clause\">{}</span>", escaped),
            TokenType::KeywordReserved => format!("<span class=\"sql-keyword-reserved\">{}</span>", escaped),
            TokenType::KeywordUnreserved => format!("<span class=\"sql-keyword-unreserved\">{}</span>", escaped),
            TokenType::KeywordType => format!("<span class=\"sql-keyword-type\">{}</span>", escaped),
            TokenType::KeywordFunction => format!("<span class=\"sql-keyword-function\">{}</span>", escaped),
            TokenType::KeywordCommon => format!("<span class=\"sql-keyword-common\">{}</span>", escaped),
            TokenType::Table => format!("<span class=\"sql-table\">{}</span>", escaped),
            TokenType::Column => format!("<span class=\"sql-column\">{}</span>", escaped),
            TokenType::String => format!("<span class=\"sql-string\">{}</span>", escaped),
            TokenType::Number => format!("<span class=\"sql-number\">{}</span>", escaped),
            TokenType::Operator => format!("<span class=\"sql-operator\">{}</span>", escaped),
            TokenType::Comment => format!("<span class=\"sql-comment\">{}</span>", escaped),
            TokenType::Text => escaped,
        };

        html.push_str(&wrapped);
    }

    html
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace(' ', "&nbsp;")
        .replace('\n', "<br/>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple_query() {
        let config = HighlightConfig {
            keywords: vec![
                SqlKeyword {
                    word: "SELECT".to_string(),
                    category: "reserved".to_string(),
                    description: None,
                },
            ],
            schema: None,
        };

        let html = highlight_sql("SELECT * FROM users", &config);
        assert!(html.contains("sql-keyword"));
    }
}
