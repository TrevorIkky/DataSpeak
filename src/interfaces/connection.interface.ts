import type { DatabaseType } from "@/types/database.types";

export interface IConnectionFormData {
  name: string;
  database_type: DatabaseType;
  host: string;
  port: number;
  username: string;
  password: string;
  default_database: string;
}

export interface IConnectionTestResult {
  success: boolean;
  message: string;
  error?: string;
}
