import type { Schema, SqlKeyword, Table } from "@/types/database.types";

export type AutocompleteContext =
  | { type: "keyword"; prefix: string }
  | { type: "table"; prefix: string; afterJoin: boolean }
  | { type: "column"; prefix: string; tables: string[] }
  | { type: "aliased-column"; prefix: string; alias: string; query: string }
  | { type: "none" };

export interface Suggestion {
  label: string;
  value: string;
  description?: string;
  category: "keyword" | "table" | "column" | "function";
  metadata?: {
    dataType?: string;
    isPrimaryKey?: boolean;
    isForeignKey?: boolean;
    rowCount?: number;
  };
}

/**
 * Detects the context at the cursor position to determine what type of suggestions to show
 */
export function detectContext(text: string, cursorPos: number): AutocompleteContext {
  const beforeCursor = text.substring(0, cursorPos);

  // Check if we're inside a string literal or comment
  if (isInsideStringOrComment(beforeCursor)) {
    return { type: "none" };
  }

  // Get the current word being typed
  const words = beforeCursor.split(/\s+/);
  const lastWord = words[words.length - 1] || "";

  // Remove any trailing punctuation from the prefix
  const prefix = lastWord.replace(/[,;()]+$/, "");

  // Check if we're typing after an alias (e.g., "tc.")
  const aliasMatch = prefix.match(/^(\w+)\.(\w*)$/);
  if (aliasMatch) {
    const [, alias, columnPrefix] = aliasMatch;
    return {
      type: "aliased-column",
      prefix: columnPrefix,
      alias,
      query: text
    };
  }

  // Check if we're after FROM or JOIN keywords (table context)
  const fromMatch = /\bFROM\s+(\w*)$/i.test(beforeCursor);
  const joinMatch = /\b(JOIN|INNER\s+JOIN|LEFT\s+JOIN|RIGHT\s+JOIN|OUTER\s+JOIN|CROSS\s+JOIN)\s+(\w*)$/i.test(beforeCursor);

  if (fromMatch || joinMatch) {
    return {
      type: "table",
      prefix,
      afterJoin: joinMatch
    };
  }

  // Check if we're after SELECT (column context)
  const selectMatch = /\bSELECT\s+(?:(?!FROM).)*$/is.test(beforeCursor);
  if (selectMatch && !/\bFROM\b/i.test(beforeCursor)) {
    const tablesInQuery = extractTablesFromQuery(text);
    return {
      type: "column",
      prefix,
      tables: tablesInQuery
    };
  }

  // Check if we're after WHERE, ON, HAVING (column context with tables from query)
  const whereMatch = /\b(WHERE|ON|HAVING|AND|OR)\s+(\w*)$/i.test(beforeCursor);
  if (whereMatch) {
    const tablesInQuery = extractTablesFromQuery(beforeCursor);
    return {
      type: "column",
      prefix,
      tables: tablesInQuery
    };
  }

  // Default to keyword context
  return {
    type: "keyword",
    prefix
  };
}

/**
 * Checks if the cursor is inside a string literal or comment
 */
function isInsideStringOrComment(text: string): boolean {
  let inSingleQuote = false;
  let inDoubleQuote = false;
  let inLineComment = false;
  let inBlockComment = false;

  for (let i = 0; i < text.length; i++) {
    const char = text[i];
    const nextChar = text[i + 1];

    // Handle comments
    if (!inSingleQuote && !inDoubleQuote) {
      if (char === '-' && nextChar === '-') {
        inLineComment = true;
      } else if (char === '/' && nextChar === '*') {
        inBlockComment = true;
      } else if (char === '*' && nextChar === '/') {
        inBlockComment = false;
        i++; // Skip next char
        continue;
      } else if (char === '\n') {
        inLineComment = false;
      }
    }

    // Handle string literals
    if (!inLineComment && !inBlockComment) {
      if (char === "'" && text[i - 1] !== '\\') {
        inSingleQuote = !inSingleQuote;
      } else if (char === '"' && text[i - 1] !== '\\') {
        inDoubleQuote = !inDoubleQuote;
      }
    }
  }

  return inSingleQuote || inDoubleQuote || inLineComment || inBlockComment;
}

/**
 * Extracts table names from the query (FROM and JOIN clauses)
 */
function extractTablesFromQuery(query: string): string[] {
  const tables: string[] = [];

  // Match FROM clause
  const fromRegex = /\bFROM\s+(\w+)/gi;
  let match;
  while ((match = fromRegex.exec(query)) !== null) {
    tables.push(match[1]);
  }

  // Match JOIN clauses
  const joinRegex = /\b(?:INNER\s+|LEFT\s+|RIGHT\s+|OUTER\s+|CROSS\s+)?JOIN\s+(\w+)/gi;
  while ((match = joinRegex.exec(query)) !== null) {
    tables.push(match[1]);
  }

  return tables;
}

/**
 * Extracts table aliases from the query
 * Returns a map of alias -> table name
 */
function extractTableAliases(query: string, keywords: SqlKeyword[]): Map<string, string> {
  const aliasMap = new Map<string, string>();

  // Build keyword set for checking
  const keywordSet = new Set(keywords.map(k => k.word.toUpperCase()));

  // Match: FROM table AS alias or FROM table alias
  const fromRegex = /\bFROM\s+(\w+)(?:\s+AS\s+|\s+)(\w+)/gi;
  let match;
  while ((match = fromRegex.exec(query)) !== null) {
    const [, tableName, alias] = match;
    // Only add if alias is not a SQL keyword
    if (!keywordSet.has(alias.toUpperCase())) {
      aliasMap.set(alias.toLowerCase(), tableName);
    }
  }

  // Match: JOIN table AS alias or JOIN table alias
  const joinRegex = /\b(?:INNER\s+|LEFT\s+|RIGHT\s+|OUTER\s+|CROSS\s+)?JOIN\s+(\w+)(?:\s+AS\s+|\s+)(\w+)/gi;
  while ((match = joinRegex.exec(query)) !== null) {
    const [, tableName, alias] = match;
    if (!keywordSet.has(alias.toUpperCase())) {
      aliasMap.set(alias.toLowerCase(), tableName);
    }
  }

  return aliasMap;
}

/**
 * Generates suggestions based on the context
 */
export function generateSuggestions(
  context: AutocompleteContext,
  schema: Schema | null,
  keywords: SqlKeyword[]
): Suggestion[] {
  switch (context.type) {
    case "keyword":
      return generateKeywordSuggestions(context.prefix, keywords);

    case "table":
      return generateTableSuggestions(context.prefix, schema, context.afterJoin);

    case "column":
      return generateColumnSuggestions(context.prefix, schema, context.tables);

    case "aliased-column":
      return generateAliasedColumnSuggestions(context.prefix, context.alias, context.query, schema, keywords);

    default:
      return [];
  }
}

/**
 * Generates keyword suggestions
 */
function generateKeywordSuggestions(prefix: string, keywords: SqlKeyword[]): Suggestion[] {
  const lowerPrefix = prefix.toLowerCase();

  return keywords
    .filter((kw) => kw.word.toLowerCase().startsWith(lowerPrefix))
    .map((kw) => ({
      label: kw.word,
      value: kw.word,
      description: kw.description || kw.category,
      category: "keyword" as const,
    }))
    .slice(0, 50); // Limit to 50 suggestions
}

/**
 * Generates table suggestions
 */
function generateTableSuggestions(
  prefix: string,
  schema: Schema | null,
  afterJoin: boolean
): Suggestion[] {
  if (!schema) return [];

  const lowerPrefix = prefix.toLowerCase();
  let tables = schema.tables.filter((table) =>
    table.name.toLowerCase().startsWith(lowerPrefix)
  );

  // If after JOIN, prioritize tables with foreign key relationships
  if (afterJoin) {
    tables = prioritizeRelatedTables(tables, schema);
  }

  return tables.map((table) => ({
    label: table.name,
    value: table.name,
    description: table.row_count ? `${table.row_count.toLocaleString()} rows` : undefined,
    category: "table" as const,
    metadata: {
      rowCount: table.row_count,
    },
  }));
}

/**
 * Prioritizes tables that have foreign key relationships with tables already in the query
 */
function prioritizeRelatedTables(tables: Table[], schema: Schema): Table[] {
  const relatedTables: Table[] = [];
  const unrelatedTables: Table[] = [];

  for (const table of tables) {
    const hasForeignKeys = table.columns.some((col) => col.is_foreign_key);
    const isReferencedByOthers = schema.tables.some((t) =>
      t.columns.some((c) => c.foreign_key_table === table.name)
    );

    if (hasForeignKeys || isReferencedByOthers) {
      relatedTables.push(table);
    } else {
      unrelatedTables.push(table);
    }
  }

  return [...relatedTables, ...unrelatedTables];
}

/**
 * Generates column suggestions
 */
function generateColumnSuggestions(
  prefix: string,
  schema: Schema | null,
  tablesInQuery: string[]
): Suggestion[] {
  if (!schema) return [];

  const lowerPrefix = prefix.toLowerCase();
  const suggestions: Suggestion[] = [];

  // If specific tables are mentioned in the query, only suggest columns from those tables
  const tablesToSearch = tablesInQuery.length > 0
    ? schema.tables.filter((t) =>
        tablesInQuery.some((queryTable) =>
          queryTable.toLowerCase() === t.name.toLowerCase()
        )
      )
    : schema.tables; // If no tables in query yet, suggest from all tables

  for (const table of tablesToSearch) {
    for (const column of table.columns) {
      if (column.name.toLowerCase().startsWith(lowerPrefix)) {
        const keyIndicators = [];
        if (column.is_primary_key) keyIndicators.push("🔑");
        if (column.is_foreign_key) keyIndicators.push("🔗");

        const description = [
          column.data_type,
          ...keyIndicators,
          tablesInQuery.length > 1 ? `(${table.name})` : "",
        ]
          .filter(Boolean)
          .join(" ");

        suggestions.push({
          label: column.name,
          value: column.name,
          description,
          category: "column" as const,
          metadata: {
            dataType: column.data_type,
            isPrimaryKey: column.is_primary_key,
            isForeignKey: column.is_foreign_key,
          },
        });
      }
    }
  }

  return suggestions.slice(0, 50); // Limit to 50 suggestions
}

/**
 * Generates column suggestions for aliased table (e.g., tc.COLUMN_NAME)
 */
function generateAliasedColumnSuggestions(
  prefix: string,
  alias: string,
  query: string,
  schema: Schema | null,
  keywords: SqlKeyword[]
): Suggestion[] {
  if (!schema) return [];

  // Extract table aliases from the query
  const aliasMap = extractTableAliases(query, keywords);
  const tableName = aliasMap.get(alias.toLowerCase());

  if (!tableName) {
    // If we can't resolve the alias, return empty
    return [];
  }

  // Find the table in the schema
  const table = schema.tables.find(
    (t) => t.name.toLowerCase() === tableName.toLowerCase()
  );

  if (!table) return [];

  // Generate suggestions for columns in this specific table
  const lowerPrefix = prefix.toLowerCase();
  const suggestions: Suggestion[] = [];

  for (const column of table.columns) {
    if (column.name.toLowerCase().startsWith(lowerPrefix)) {
      const keyIndicators = [];
      if (column.is_primary_key) keyIndicators.push("🔑");
      if (column.is_foreign_key) keyIndicators.push("🔗");

      const description = [
        column.data_type,
        ...keyIndicators,
        `(${table.name})`,
      ]
        .filter(Boolean)
        .join(" ");

      suggestions.push({
        label: column.name,
        value: column.name,
        description,
        category: "column" as const,
        metadata: {
          dataType: column.data_type,
          isPrimaryKey: column.is_primary_key,
          isForeignKey: column.is_foreign_key,
        },
      });
    }
  }

  return suggestions;
}

/**
 * Inserts suggestion at cursor position
 */
export function insertSuggestion(
  text: string,
  cursorPos: number,
  suggestion: string
): { newText: string; newCursorPos: number } {
  const beforeCursor = text.substring(0, cursorPos);
  const afterCursor = text.substring(cursorPos);

  // Find the start of the current word
  const words = beforeCursor.split(/\s+/);
  const lastWord = words[words.length - 1] || "";

  // Check if we're completing an aliased column (e.g., tc.COL)
  const aliasMatch = lastWord.match(/^(\w+\.)(\w*)$/);
  if (aliasMatch) {
    const [, aliasPrefix, ] = aliasMatch;
    const wordStart = cursorPos - lastWord.length;
    // Keep the alias prefix and replace just the column part
    const newText = text.substring(0, wordStart) + aliasPrefix + suggestion + afterCursor;
    const newCursorPos = wordStart + aliasPrefix.length + suggestion.length;
    return { newText, newCursorPos };
  }

  // Normal completion
  const wordStart = cursorPos - lastWord.length;
  const newText = text.substring(0, wordStart) + suggestion + afterCursor;
  const newCursorPos = wordStart + suggestion.length;

  return { newText, newCursorPos };
}
