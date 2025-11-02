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
  indexes: Index[];
  triggers: Trigger[];
  constraints: Constraint[];
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

export type Index = {
  name: string;
  columns: string[];
  is_unique: boolean;
  is_primary: boolean;
  index_type?: string;
};

export type Trigger = {
  name: string;
  event: string;
  timing: string;
  statement?: string;
};

export type Constraint = {
  name: string;
  constraint_type: string;
  columns: string[];
  referenced_table?: string;
  referenced_columns?: string[];
};

export type Schema = {
  database_name: string;
  tables: Table[];
};

export type SqlKeyword = {
  word: string;
  category: string;
  description?: string;
};

export type ConnectionStatus = "connected" | "disconnected" | "connecting" | "error";
