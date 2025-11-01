import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Schema, Table } from "@/types/database.types";
import type { ISchemaStore } from "@/interfaces/store.interface";
import { ErrorHandler } from "@/lib/ErrorHandler";

interface SchemaLoadProgress {
  table: Table;
  loaded: number;
  total: number;
}

export const useSchemaStore = create<ISchemaStore>((set, get) => ({
  schema: null,
  isLoading: false,
  error: null,

  loadSchema: async (connectionId: string) => {
    set({ isLoading: true, error: null, schema: null });

    // Listen for progressive schema loading events
    const unlisten = await listen<SchemaLoadProgress>("schema-load-progress", (event) => {
      const { table } = event.payload;

      const currentSchema = get().schema;
      if (!currentSchema) {
        // First table - create new schema
        set({
          schema: {
            database_name: "",
            tables: [table],
          },
        });
      } else {
        // Add table to existing schema
        set({
          schema: {
            ...currentSchema,
            tables: [...currentSchema.tables, table],
          },
        });
      }
    });

    try {
      const schema = await invoke<Schema>("get_schema", { connectionId });

      // Update with final schema (includes database name)
      set({
        schema: {
          ...schema,
          tables: get().schema?.tables || schema.tables,
        },
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

  clearSchema: () => {
    set({ schema: null, error: null });
  },
}));
