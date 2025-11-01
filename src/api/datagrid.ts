import { invoke } from "@tauri-apps/api/core";
import type { DataGridChanges } from "@/types/datagrid.types";

export interface CommitRequest {
  connection_id: string;
  table_name: string;
  primary_key_columns: string[];
  changes: {
    edits: Array<{
      row_index: number;
      column_name: string;
      old_value: any;
      new_value: any;
    }>;
    deletes: number[];
    inserts: Array<{
      temp_id: string;
      row_data: Record<string, any>;
    }>;
  };
  original_rows: Record<string, any>[];
}

export interface CommitResult {
  success: boolean;
  message: string;
  edits_count: number;
  deletes_count: number;
  inserts_count: number;
}

export async function commitDataChanges(
  connectionId: string,
  tableName: string,
  primaryKeyColumns: string[],
  changes: DataGridChanges,
  originalRows: Record<string, any>[]
): Promise<CommitResult> {
  // Convert Map to array of edits and transform to match backend format
  const edits = Array.from(changes.edits.values()).map(edit => ({
    row_index: edit.rowIndex,
    column_name: edit.columnName,
    old_value: edit.oldValue,
    new_value: edit.newValue,
  }));

  // Convert Set to array of deletes
  const deletes = Array.from(changes.deletes);

  // Transform inserts to match backend format
  const inserts = changes.inserts.map(insert => ({
    temp_id: insert.tempId,
    row_data: insert.rowData,
  }));

  const request: CommitRequest = {
    connection_id: connectionId,
    table_name: tableName,
    primary_key_columns: primaryKeyColumns,
    changes: {
      edits,
      deletes,
      inserts,
    },
    original_rows: originalRows,
  };

  return invoke<CommitResult>("commit_data_changes", { request });
}
