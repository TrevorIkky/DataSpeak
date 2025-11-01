// Data grid editing types

export type ChangeType = "edit" | "delete" | "insert";

export interface CellEdit {
  rowIndex: number;
  columnName: string;
  oldValue: any;
  newValue: any;
}

export interface RowDelete {
  rowIndex: number;
  rowData: Record<string, any>;
}

export interface RowInsert {
  tempId: string; // Temporary ID for tracking before commit
  rowData: Record<string, any>;
}

export interface DataGridChanges {
  edits: Map<string, CellEdit>; // Key: `${rowIndex}-${columnName}`
  deletes: Set<number>; // Row indices
  inserts: RowInsert[];
}

export interface EditingState {
  isEditing: boolean;
  editingCell: { rowIndex: number; columnName: string } | null;
}
