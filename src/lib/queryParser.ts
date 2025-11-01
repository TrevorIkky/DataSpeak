/**
 * Simple SQL query parser to extract table information
 * This handles basic SELECT queries only
 */

export interface QueryTableInfo {
  tableName: string | null;
  isSimpleQuery: boolean; // True if it's a simple single-table query
}

/**
 * Attempts to extract the table name from a simple SELECT query
 * Returns null if the query is complex (JOIN, subquery, etc.)
 */
export function extractTableFromQuery(query: string): QueryTableInfo {
  // Normalize the query
  const normalizedQuery = query
    .trim()
    .replace(/\s+/g, " ") // Replace multiple spaces with single space
    .replace(/\n/g, " ") // Replace newlines with space
    .toUpperCase();

  // Check if it's a SELECT query
  if (!normalizedQuery.startsWith("SELECT")) {
    return { tableName: null, isSimpleQuery: false };
  }

  // Check for complex features that we don't support
  const complexPatterns = [
    /\bJOIN\b/i,
    /\bUNION\b/i,
    /\(\s*SELECT\b/i, // Subquery
    /,\s*\(/i, // Subquery in column list
  ];

  for (const pattern of complexPatterns) {
    if (pattern.test(normalizedQuery)) {
      return { tableName: null, isSimpleQuery: false };
    }
  }

  // Try to extract FROM clause
  // Pattern: SELECT ... FROM tablename ...
  const fromMatch = normalizedQuery.match(/\bFROM\s+(["`]?)(\w+)\1/i);

  if (!fromMatch) {
    return { tableName: null, isSimpleQuery: false };
  }

  const tableName = fromMatch[2];

  // Check if there are multiple FROM clauses (shouldn't happen in valid SQL, but check anyway)
  const fromCount = (normalizedQuery.match(/\bFROM\b/gi) || []).length;
  if (fromCount > 1) {
    return { tableName: null, isSimpleQuery: false };
  }

  return {
    tableName: tableName.toLowerCase(),
    isSimpleQuery: true,
  };
}
