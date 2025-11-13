import type {
  Connection,
  Schema,
  QueryResult
} from "@/types/database.types";
import type { AppSettings } from "@/types/settings.types";
import type { ERDData } from "@/types/erd.types";
import type {
  ExportResult,
  ImportResult
} from "@/types/export.types";
import type { VisualizationConfig } from "@/types/ai.types";

// Tauri command interfaces
export interface IRustCommands {
  // Settings
  save_settings(settings: AppSettings): Promise<void>;
  get_settings(): Promise<AppSettings | null>;

  // Connections
  test_connection(connection: Partial<Connection>): Promise<{ success: boolean; message: string }>;
  save_connection(connection: Partial<Connection>): Promise<Connection>;
  get_connections(): Promise<Connection[]>;
  delete_connection(id: string): Promise<void>;
  update_connection(connection: Connection): Promise<Connection>;

  // Schema & Query
  get_schema(connection_id: string): Promise<Schema>;
  run_query(connection_id: string, query: string, limit: number, offset: number): Promise<QueryResult>;

  // Import/Export
  export_database(connection_id: string, database_name: string): Promise<ExportResult>;
  import_database(connection_id: string, zip_path: string): Promise<ImportResult>;

  // AI
  generate_sql(prompt: string, schema: string, model: string): Promise<string>;
  generate_visualization(data: Record<string, any>[], prompt: string, model: string): Promise<VisualizationConfig>;

  // ERD
  get_erd_data(connection_id: string, database_name: string): Promise<ERDData>;
}
