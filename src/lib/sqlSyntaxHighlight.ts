/**
 * SQL Syntax Highlighting Utility
 * Calls Rust backend for syntax highlighting
 */

import { invoke } from "@tauri-apps/api/core";
import type { Schema, SqlKeyword } from "@/types/database.types";

export interface HighlightConfig {
  keywords: SqlKeyword[];
  schema: Schema | null;
}

/**
 * Convert SQL text to highlighted HTML using Rust backend
 */
export async function highlightSQL(sql: string, config: HighlightConfig): Promise<string> {
  try {
    const html = await invoke<string>("highlight_sql", { sql, config });
    return html;
  } catch (error) {
    console.error("Failed to highlight SQL:", error);
    // Fallback: return escaped text without highlighting
    return sql
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/ /g, '&nbsp;')
      .replace(/\n/g, '<br/>');
  }
}
