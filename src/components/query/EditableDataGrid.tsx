import { useMemo, useState, useRef, useEffect } from "react";
import {
  useReactTable,
  getCoreRowModel,
  getPaginationRowModel,
  flexRender,
  ColumnDef,
} from "@tanstack/react-table";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
  MapPin,
  Trash2,
  Plus,
  Save,
  RotateCcw,
} from "lucide-react";
import type { QueryResult } from "@/types/query.types";
import { isWKTGeometry, parseWKT } from "@/lib/geoUtils";
import type { GeographicCell } from "@/types/geography.types";
import type { DataGridChanges, RowInsert } from "@/types/datagrid.types";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

interface EditableDataGridProps {
  result: QueryResult;
  onGeographicCellClick?: (cell: GeographicCell) => void;
  onCommitChanges?: (changes: DataGridChanges, originalRows: Record<string, any>[]) => Promise<void>;
  tableName?: string;
  primaryKeyColumns?: string[];
}

export function EditableDataGrid({
  result,
  onGeographicCellClick,
  onCommitChanges,
  tableName,
  primaryKeyColumns,
}: EditableDataGridProps) {
  const [pagination, setPagination] = useState({
    pageIndex: 0,
    pageSize: 50,
  });

  // Track changes
  const [changes, setChanges] = useState<DataGridChanges>({
    edits: new Map(),
    deletes: new Set(),
    inserts: [],
  });

  // Editing state
  const [editingCell, setEditingCell] = useState<{ rowIndex: number; columnName: string } | null>(
    null
  );
  const [editValue, setEditValue] = useState<string>("");
  const inputRef = useRef<HTMLInputElement>(null);
  const [selectedRows, setSelectedRows] = useState<Set<number>>(new Set());
  const clickTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const saveEditRef = useRef<(() => void) | null>(null);

  // Focus input when editing starts
  useEffect(() => {
    if (editingCell && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editingCell]);

  // Cleanup click timeout on unmount
  useEffect(() => {
    return () => {
      if (clickTimeoutRef.current) {
        clearTimeout(clickTimeoutRef.current);
      }
    };
  }, []);

  // Handle clicks outside of editing cell to save and exit
  useEffect(() => {
    if (!editingCell) return;

    const handleClickOutside = (event: MouseEvent) => {
      // Check if click is outside the input
      if (inputRef.current && !inputRef.current.contains(event.target as Node)) {
        // Use the ref to avoid stale closure issues
        saveEditRef.current?.();
      }
    };

    // Use mousedown instead of click to capture before other handlers
    document.addEventListener('mousedown', handleClickOutside);

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [editingCell]);

  // Combine original data with inserts and apply deletes
  const displayData = useMemo(() => {
    const data = [...result.rows, ...changes.inserts.map((insert) => insert.rowData)];
    return data.filter((_, idx) => !changes.deletes.has(idx));
  }, [result.rows, changes.inserts, changes.deletes]);

  // Get change key for cell
  const getChangeKey = (rowIndex: number, columnName: string) => {
    return `${rowIndex}-${columnName}`;
  };

  // Check if cell is edited
  const isCellEdited = (rowIndex: number, columnName: string) => {
    return changes.edits.has(getChangeKey(rowIndex, columnName));
  };

  // Check if row is deleted
  const isRowDeleted = (rowIndex: number) => {
    return changes.deletes.has(rowIndex);
  };

  // Check if row is newly inserted
  const isRowInserted = (rowIndex: number) => {
    return rowIndex >= result.rows.length;
  };

  // Start editing a cell
  const startEditing = (rowIndex: number, columnName: string, currentValue: any) => {
    setEditingCell({ rowIndex, columnName });
    setEditValue(currentValue === null ? "" : String(currentValue));
  };

  // Cancel editing
  const cancelEditing = () => {
    setEditingCell(null);
    setEditValue("");
  };

  // Save cell edit (auto-save on blur)
  const saveEdit = () => {
    if (!editingCell) return;

    const { rowIndex, columnName } = editingCell;
    const oldValue = result.rows[rowIndex]?.[columnName];
    const newValue = editValue === "" ? null : editValue;

    if (oldValue !== newValue) {
      const key = getChangeKey(rowIndex, columnName);
      const newEdits = new Map(changes.edits);
      newEdits.set(key, { rowIndex, columnName, oldValue, newValue });
      setChanges({ ...changes, edits: newEdits });
    }

    cancelEditing();
  };

  // Keep ref updated with latest saveEdit function
  useEffect(() => {
    saveEditRef.current = saveEdit;
  });

  // Toggle row selection
  const toggleRowSelection = (rowIndex: number) => {
    const newSelection = new Set(selectedRows);
    if (newSelection.has(rowIndex)) {
      newSelection.delete(rowIndex);
    } else {
      newSelection.add(rowIndex);
    }
    setSelectedRows(newSelection);
  };

  // Delete selected rows
  const deleteSelectedRows = () => {
    const newDeletes = new Set(changes.deletes);
    selectedRows.forEach((rowIndex) => {
      newDeletes.add(rowIndex);
    });
    setChanges({ ...changes, deletes: newDeletes });
    setSelectedRows(new Set()); // Clear selection after delete
  };

  // Add new row
  const addRow = () => {
    const tempId = `temp-${Date.now()}`;
    const newRow: Record<string, any> = {};
    result.columns.forEach((col) => {
      newRow[col] = null;
    });

    const newInsert: RowInsert = {
      tempId,
      rowData: newRow,
    };

    setChanges({
      ...changes,
      inserts: [...changes.inserts, newInsert],
    });
  };

  // Reset all changes
  const resetChanges = () => {
    setChanges({
      edits: new Map(),
      deletes: new Set(),
      inserts: [],
    });
    cancelEditing();
  };

  // Check if commits are supported
  const canCommit = tableName && primaryKeyColumns && primaryKeyColumns.length > 0;

  // Commit changes
  const handleCommit = async () => {
    if (onCommitChanges && canCommit) {
      await onCommitChanges(changes, result.rows);
      resetChanges();
    }
  };

  // Calculate totals
  const totalChanges = changes.edits.size + changes.deletes.size + changes.inserts.length;
  const hasChanges = totalChanges > 0;

  // Helper function to render cell values based on type
  const renderCellValue = (value: any, columnName: string, rowIndex: number) => {
    // Check if this cell is being edited
    if (editingCell?.rowIndex === rowIndex && editingCell?.columnName === columnName) {
      return (
        <Input
          ref={inputRef}
          value={editValue}
          onChange={(e) => setEditValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              saveEdit();
            }
            if (e.key === "Escape") {
              e.preventDefault();
              cancelEditing();
            }
          }}
          onClick={(e) => {
            // Prevent clicks on the input from bubbling up
            e.stopPropagation();
          }}
          onBlur={saveEdit}
          className="h-7 text-sm w-full"
        />
      );
    }

    // Get the actual value (might be edited)
    const changeKey = getChangeKey(rowIndex, columnName);
    const editedValue = changes.edits.get(changeKey);
    const displayValue = editedValue ? editedValue.newValue : value;

    // Handle NULL
    if (displayValue === null || displayValue === undefined) {
      return <span className="text-muted-foreground italic whitespace-nowrap">NULL</span>;
    }

    // Handle boolean
    if (typeof displayValue === "boolean") {
      return (
        <span className="font-mono px-2 py-0.5 rounded bg-blue-500/10 text-blue-700 dark:text-blue-400 whitespace-nowrap">
          {displayValue.toString()}
        </span>
      );
    }

    // Handle number
    if (typeof displayValue === "number") {
      return (
        <span className="font-mono text-purple-700 dark:text-purple-400 whitespace-nowrap">
          {displayValue.toLocaleString()}
        </span>
      );
    }

    // Handle string
    if (typeof displayValue === "string") {
      // Check for WKT geometry data (PostGIS/MySQL Spatial)
      if (isWKTGeometry(displayValue)) {
        const geometry = parseWKT(displayValue);
        const displayText =
          displayValue.length > 40 ? displayValue.substring(0, 40) + "..." : displayValue;

        return (
          <button
            onClick={(e) => {
              e.stopPropagation();
              if (geometry && onGeographicCellClick) {
                onGeographicCellClick({
                  columnName,
                  rowIndex,
                  geometry,
                  rawValue: displayValue,
                });
              }
            }}
            onDoubleClick={(e) => {
              // Prevent double-click from triggering cell edit for geographic cells
              e.stopPropagation();
            }}
            className="flex items-center gap-2 font-mono text-green-700 dark:text-green-400 text-sm hover:underline cursor-pointer whitespace-nowrap"
            title={`Click to view on map\n\n${displayValue}`}
          >
            <MapPin className="h-3 w-3 flex-shrink-0" />
            <span className="truncate max-w-[200px]">{displayText}</span>
          </button>
        );
      }

      // Check for date/timestamp patterns (ISO 8601 format)
      const datePattern = /^\d{4}-\d{2}-\d{2}$/;
      const timePattern = /^\d{2}:\d{2}:\d{2}/;
      const dateTimePattern = /^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}/;

      if (dateTimePattern.test(displayValue) || datePattern.test(displayValue) || timePattern.test(displayValue)) {
        return (
          <span className="font-mono text-green-700 dark:text-green-400 whitespace-nowrap" title={displayValue}>
            {displayValue}
          </span>
        );
      }

      // Check for hex binary data
      if (displayValue.startsWith("0x")) {
        const displayText =
          displayValue.length > 50 ? displayValue.substring(0, 50) + "..." : displayValue;
        return (
          <span className="font-mono text-orange-700 dark:text-orange-400 text-xs whitespace-nowrap" title={displayValue}>
            {displayText}
          </span>
        );
      }

      // Check for UUID pattern
      const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
      if (uuidPattern.test(displayValue)) {
        return (
          <span className="font-mono text-cyan-700 dark:text-cyan-400 text-sm whitespace-nowrap" title={displayValue}>
            {displayValue}
          </span>
        );
      }

      // Regular string - single line with truncation
      return (
        <span className="block truncate max-w-[300px] whitespace-nowrap" title={displayValue}>
          {displayValue}
        </span>
      );
    }

    // Handle object (like JSON)
    if (typeof displayValue === "object") {
      const jsonStr = JSON.stringify(displayValue);
      return (
        <span
          className="font-mono text-sm text-indigo-700 dark:text-indigo-400 block truncate max-w-[300px] whitespace-nowrap"
          title={jsonStr}
        >
          {jsonStr}
        </span>
      );
    }

    // Fallback
    return (
      <span className="truncate block max-w-[300px] whitespace-nowrap" title={String(displayValue)}>
        {String(displayValue)}
      </span>
    );
  };

  // Create columns from query result
  const columns = useMemo<ColumnDef<Record<string, any>>[]>(() => {
    const dataColumns: ColumnDef<Record<string, any>>[] = [
      // Checkbox column for row selection
      {
        id: "select",
        header: ({ table }) => (
          <div className="flex items-center justify-center">
            {displayData.length > 0 && (
              <input
                type="checkbox"
                checked={table.getIsAllPageRowsSelected()}
                onChange={table.getToggleAllPageRowsSelectedHandler()}
                className="h-4 w-4 rounded border-gray-300"
              />
            )}
          </div>
        ),
        cell: ({ row }) => {
          const rowIndex = row.index;
          const isDeleted = isRowDeleted(rowIndex);
          return (
            <div className="flex items-center justify-center py-2">
              <input
                type="checkbox"
                checked={selectedRows.has(rowIndex)}
                onChange={() => toggleRowSelection(rowIndex)}
                disabled={isDeleted}
                className="h-4 w-4 rounded border-gray-300"
              />
            </div>
          );
        },
      },
      // Data columns
      ...result.columns.map((columnName) => ({
        accessorKey: columnName,
        header: columnName,
        cell: ({ getValue, row }: any) => {
          const rowIndex = row.index;
          const value = getValue();
          const isEdited = isCellEdited(rowIndex, columnName);

          const isEditing = editingCell?.rowIndex === rowIndex && editingCell?.columnName === columnName;

          return (
            <div
              className={cn(
                "group relative py-2 px-3 overflow-hidden cursor-cell hover:bg-muted/50 transition-colors",
                isEdited && "bg-blue-50 dark:bg-blue-950/30",
                isRowDeleted(rowIndex) && "opacity-50 cursor-not-allowed",
                isRowInserted(rowIndex) && "bg-green-50 dark:bg-green-950/30",
                isEditing && "cursor-text"
              )}
              onClick={() => {
                // Don't trigger edit mode if already editing
                if (isEditing || isRowDeleted(rowIndex)) return;

                // Clear any existing timeout
                if (clickTimeoutRef.current) {
                  clearTimeout(clickTimeoutRef.current);
                  clickTimeoutRef.current = null;
                }

                // Set a timeout for single click - will be cleared if double-click happens
                clickTimeoutRef.current = setTimeout(() => {
                  startEditing(rowIndex, columnName, value);
                  clickTimeoutRef.current = null;
                }, 250);
              }}
              onDoubleClick={(e) => {
                if (!isRowDeleted(rowIndex) && !isEditing) {
                  e.stopPropagation();

                  // Clear single click timeout
                  if (clickTimeoutRef.current) {
                    clearTimeout(clickTimeoutRef.current);
                    clickTimeoutRef.current = null;
                  }

                  startEditing(rowIndex, columnName, value);
                }
              }}
              title={isRowDeleted(rowIndex) ? "Row is deleted" : isEditing ? "" : "Click to edit"}
            >
              <div className="overflow-x-auto whitespace-nowrap">
                {renderCellValue(value, columnName, rowIndex)}
              </div>
            </div>
          );
        },
      })),
    ];

    return dataColumns;
  }, [result.columns, changes, editingCell, editValue, onGeographicCellClick, selectedRows, displayData.length]);

  const table = useReactTable({
    data: displayData,
    columns,
    getCoreRowModel: getCoreRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    onPaginationChange: setPagination,
    state: {
      pagination,
    },
  });

  return (
    <div className="flex flex-col h-full">
      {/* Header with results info */}
      <div className="flex items-center justify-between px-4 py-2 border-b bg-card">
        <div className="flex items-center gap-3">
          <span className="text-sm font-semibold">
            {result.row_count} row{result.row_count !== 1 ? "s" : ""}
          </span>
          <span className="text-sm text-muted-foreground">in {result.execution_time_ms}ms</span>
          {hasChanges && (
            <div className="flex items-center gap-2 ml-4">
              <Badge variant="secondary" className="text-xs">
                {changes.edits.size} edited
              </Badge>
              <Badge variant="destructive" className="text-xs">
                {changes.deletes.size} deleted
              </Badge>
              <Badge variant="default" className="text-xs bg-green-600">
                {changes.inserts.length} added
              </Badge>
            </div>
          )}
        </div>
      </div>

      {/* Table */}
      <div className="flex-1 overflow-auto relative">
        <Table>
          <TableHeader className="sticky top-0 bg-card z-10 shadow-sm">
            {table.getHeaderGroups().map((headerGroup) => (
              <TableRow key={headerGroup.id}>
                {headerGroup.headers.map((header) => (
                  <TableHead key={header.id} className="font-semibold">
                    {header.isPlaceholder
                      ? null
                      : flexRender(header.column.columnDef.header, header.getContext())}
                  </TableHead>
                ))}
              </TableRow>
            ))}
          </TableHeader>
          <TableBody>
            {table.getRowModel().rows?.length ? (
              table.getRowModel().rows.map((row) => {
                const rowIndex = row.index;
                const isDeleted = isRowDeleted(rowIndex);
                const isNew = isRowInserted(rowIndex);

                return (
                  <TableRow
                    key={row.id}
                    data-state={row.getIsSelected() && "selected"}
                    className={cn(
                      "hover:bg-muted/50",
                      isDeleted && "line-through opacity-60 bg-red-50 dark:bg-red-950/20",
                      isNew && "bg-green-50 dark:bg-green-950/20"
                    )}
                  >
                    {row.getVisibleCells().map((cell) => (
                      <TableCell key={cell.id} className={cell.column.id === "actions" ? "py-2" : "p-0"}>
                        {flexRender(cell.column.columnDef.cell, cell.getContext())}
                      </TableCell>
                    ))}
                  </TableRow>
                );
              })
            ) : (
              <TableRow className="hover:bg-transparent">
                <TableCell colSpan={columns.length} className="h-[400px] p-0">
                  <div className="flex items-center justify-center h-full w-full">
                    <p className="text-sm text-muted-foreground">No results.</p>
                  </div>
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>

      {/* Table Actions - Above Pagination */}
      <div className="flex items-center justify-between px-4 py-2 border-t bg-card/50">
        <div className="flex items-center gap-2">
          <Button size="sm" variant="outline" onClick={addRow} className="h-8">
            <Plus className="h-4 w-4 mr-2" />
            Add Row
          </Button>
          {selectedRows.size > 0 && (
            <Button
              size="sm"
              variant="outline"
              onClick={deleteSelectedRows}
              className="h-8 text-destructive hover:text-destructive/80"
            >
              <Trash2 className="h-4 w-4 mr-2" />
              Delete Selected ({selectedRows.size})
            </Button>
          )}
        </div>

        <div className="flex items-center gap-2">
          {hasChanges && (
            <>
              <Button size="sm" variant="outline" onClick={resetChanges} className="h-8">
                <RotateCcw className="h-4 w-4 mr-2" />
                Reset
              </Button>
              <Button
                size="sm"
                onClick={handleCommit}
                className="h-8"
                disabled={!canCommit}
                title={!canCommit ? "Commits are only supported for tables with primary keys" : undefined}
              >
                <Save className="h-4 w-4 mr-2" />
                Commit ({totalChanges})
              </Button>
            </>
          )}
        </div>
      </div>

      {/* Pagination - Only show if there's data */}
      {displayData.length > 0 && (
        <div className="flex items-center justify-between px-4 py-3 border-t bg-card">
          <div className="flex items-center gap-2">
            <p className="text-sm text-muted-foreground">
              Page {table.getState().pagination.pageIndex + 1} of {table.getPageCount()}
            </p>
          </div>

          <div className="flex items-center gap-6">
            <div className="flex items-center gap-2">
              <p className="text-sm text-muted-foreground whitespace-nowrap">Rows per page</p>
              <Select
                value={`${table.getState().pagination.pageSize}`}
                onValueChange={(value) => {
                  table.setPageSize(Number(value));
                }}
              >
                <SelectTrigger className="h-8 w-[70px]">
                  <SelectValue placeholder={table.getState().pagination.pageSize} />
                </SelectTrigger>
                <SelectContent side="top">
                  {[10, 20, 50, 100, 200].map((pageSize) => (
                    <SelectItem key={pageSize} value={`${pageSize}`}>
                      {pageSize}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="flex items-center gap-1">
              <Button
                variant="outline"
                size="icon"
                className="h-8 w-8"
                onClick={() => table.firstPage()}
                disabled={!table.getCanPreviousPage()}
              >
                <ChevronsLeft className="h-4 w-4" />
              </Button>
              <Button
                variant="outline"
                size="icon"
                className="h-8 w-8"
                onClick={() => table.previousPage()}
                disabled={!table.getCanPreviousPage()}
              >
                <ChevronLeft className="h-4 w-4" />
              </Button>
              <Button
                variant="outline"
                size="icon"
                className="h-8 w-8"
                onClick={() => table.nextPage()}
                disabled={!table.getCanNextPage()}
              >
                <ChevronRight className="h-4 w-4" />
              </Button>
              <Button
                variant="outline"
                size="icon"
                className="h-8 w-8"
                onClick={() => table.lastPage()}
                disabled={!table.getCanNextPage()}
              >
                <ChevronsRight className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
