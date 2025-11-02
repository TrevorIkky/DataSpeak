export type ExportOptions = {
  connection_id: string;
  tables: string[];
  output_dir: string;
  create_zip: boolean;
};

export type ExportProgress = {
  table_name: string;
  current: number;
  total: number;
  status: string;
  cancelled: boolean;
};

export type ImportOptions = {
  connection_id: string;
  source_path: string;
  is_zip: boolean;
  table_mappings: Record<string, string>; // CSV filename -> table name
};

export type ImportProgress = {
  file_name: string;
  current: number;
  total: number;
  status: string;
  cancelled: boolean;
};
