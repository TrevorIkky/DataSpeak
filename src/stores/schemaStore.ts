import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Schema, Table, SqlKeyword } from "@/types/database.types";
import type { ISchemaStore } from "@/interfaces/store.interface";
import { ErrorHandler } from "@/lib/ErrorHandler";

interface SchemaLoadProgress {
  table: Table;
  loaded: number;
  total: number;
}

export const useSchemaStore = create<ISchemaStore>((set, get) => ({
  schema: null,
  keywords: [],
  isLoading: false,
  isLoadingKeywords: false,
  error: null,

  loadSchema: async (connectionId: string) => {
    set({ isLoading: true, error: null, schema: null });

    // Use a Map to track unique tables by name (prevents duplicates)
    const tablesMap = new Map<string, Table>();

    // Listen for progressive schema loading events
    const unlisten = await listen<SchemaLoadProgress>("schema-load-progress", (event) => {
      const { table } = event.payload;

      // Add or update table in the map (prevents duplicates)
      tablesMap.set(table.name, table);

      const currentSchema = get().schema;
      if (!currentSchema) {
        // First table - create new schema
        set({
          schema: {
            database_name: "",
            tables: Array.from(tablesMap.values()),
          },
        });
      } else {
        // Update schema with all tables from the map
        set({
          schema: {
            ...currentSchema,
            tables: Array.from(tablesMap.values()),
          },
        });
      }
    });

    try {
      const schema = await invoke<Schema>("get_schema", { connectionId });

      // Update with final schema (use the complete schema from backend)
      set({
        schema: schema,
        isLoading: false
      });

      // Clean up event listener
      unlisten();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Failed to load schema";
      set({ error: message, isLoading: false });
      ErrorHandler.handle(error, "Failed to load schema");

      // Clean up event listener
      unlisten();
    }
  },

  fetchKeywords: async (connectionId: string) => {
    set({ isLoadingKeywords: true });

    try {
      const keywords = await invoke<SqlKeyword[]>("get_sql_keywords", { connectionId });
      set({ keywords, isLoadingKeywords: false });
    } catch (error) {
      // If fetching keywords fails, just set empty array and log the error
      console.error("Failed to fetch SQL keywords:", error);
      set({ keywords: [], isLoadingKeywords: false });
    }
  },

  clearSchema: () => {
    set({ schema: null, keywords: [], error: null });
  },
}));
