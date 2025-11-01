export type DatabaseType = "PostgreSQL" | "MariaDB" | "MySQL";

export type Connection = {
  id: string;
  name: string;
  database_type: DatabaseType;
  host: string;
  port: number;
  username: string;
  password: string;
  default_database: string;
  created_at: string;
  updated_at: string;
};

export type Table = {
  name: string;
  schema?: string;
  row_count?: number;
  columns: Column[];
};

export type Column = {
  name: string;
  data_type: string;
  is_nullable: boolean;
  is_primary_key: boolean;
  is_foreign_key: boolean;
  foreign_key_table?: string;
  foreign_key_column?: string;
  default_value?: string;
  character_maximum_length?: number;
};

export type Schema = {
  database_name: string;
  tables: Table[];
};

export type ConnectionStatus = "connected" | "disconnected" | "connecting" | "error";
