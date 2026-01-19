import { useMemo, useState, useRef, useEffect, useCallback } from "react";
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
  ExternalLink,
  Filter,
  FilterX,
} from "lucide-react";
import type { QueryResult } from "@/types/query.types";
import { isWKTGeometry, parseWKT } from "@/lib/geoUtils";
import type { GeographicCell } from "@/types/geography.types";
import type { DataGridChanges, RowInsert } from "@/types/datagrid.types";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { useQueryStore } from "@/stores/queryStore";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  DialogTrigger,
} from "@/components/ui/dialog";

// Column filter types
type FilterOperator = "equals" | "contains" | "startsWith" | "endsWith" | "greaterThan" | "lessThan" | "between" | "isEmpty" | "isNotEmpty";

interface ColumnFilter {
  column: string;
  operator: FilterOperator;
  value: string;
  value2?: string; // For "between" operator
}

interface EditableDataGridProps {
  result: QueryResult;
  onGeographicCellClick?: (cell: GeographicCell) => void;
  onCommitChanges?: (changes: DataGridChanges, originalRows: Record<string, any>[]) => Promise<void>;
  tableName?: string;
  primaryKeyColumns?: string[];
}

// Filter Popover Component
interface FilterPopoverProps {
  columnName: string;
  dataType: string;
  currentFilter: ColumnFilter | undefined;
  onApplyFilter: (filter: ColumnFilter) => void;
  onRemoveFilter: () => void;
}

function FilterPopover({
  columnName,
  dataType,
  currentFilter,
  onApplyFilter,
  onRemoveFilter,
}: FilterPopoverProps) {
  const [operator, setOperator] = useState<FilterOperator>(currentFilter?.operator || "contains");
  const [value, setValue] = useState(currentFilter?.value || "");
  const [value2, setValue2] = useState(currentFilter?.value2 || "");
  const [isOpen, setIsOpen] = useState(false);

  // Determine available operators based on data type
  const getOperatorsForType = () => {
    const isNumeric = dataType?.includes("INT") || dataType?.includes("FLOAT") ||
                      dataType?.includes("DOUBLE") || dataType?.includes("DECIMAL") ||
                      dataType?.includes("NUMERIC");

    if (isNumeric) {
      return [
        { value: "equals", label: "Equals" },
        { value: "greaterThan", label: "Greater than" },
        { value: "lessThan", label: "Less than" },
        { value: "between", label: "Between" },
        { value: "isEmpty", label: "Is empty" },
        { value: "isNotEmpty", label: "Is not empty" },
      ] as const;
    }

    return [
      { value: "contains", label: "Contains" },
      { value: "equals", label: "Equals" },
      { value: "startsWith", label: "Starts with" },
      { value: "endsWith", label: "Ends with" },
      { value: "isEmpty", label: "Is empty" },
      { value: "isNotEmpty", label: "Is not empty" },
    ] as const;
  };

  const operators = getOperatorsForType();

  const handleApply = () => {
    if ((operator !== "isEmpty" && operator !== "isNotEmpty") && !value) {
      return; // Don't apply empty filters
    }

    onApplyFilter({
      column: columnName,
      operator,
      value,
      value2: operator === "between" ? value2 : undefined,
    });
    setIsOpen(false);
  };

  const handleClear = () => {
    setValue("");
    setValue2("");
    onRemoveFilter();
    setIsOpen(false);
  };

  const needsInput = operator !== "isEmpty" && operator !== "isNotEmpty";
  const needsSecondInput = operator === "between";

  return (
    <Dialog open={isOpen} onOpenChange={setIsOpen}>
      <DialogTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className={cn(
            "h-6 w-6 p-0 opacity-0 group-hover:opacity-100 transition-opacity",
            currentFilter && "opacity-100 text-blue-600 dark:text-blue-400"
          )}
          onClick={(e) => {
            e.stopPropagation();
          }}
        >
          {currentFilter ? <FilterX className="h-3.5 w-3.5" /> : <Filter className="h-3.5 w-3.5" />}
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Filter {columnName}</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-2">
            <label className="text-sm font-medium">Operator</label>
            <Select value={operator} onValueChange={(v) => setOperator(v as FilterOperator)}>
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {operators.map((op) => (
                  <SelectItem key={op.value} value={op.value}>
                    {op.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {needsInput && (
            <div className="space-y-2">
              <label className="text-sm font-medium">Value</label>
              <Input
                placeholder="Filter value..."
                value={value}
                onChange={(e) => setValue(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    handleApply();
                  }
                }}
                autoFocus
              />
              {needsSecondInput && (
                <>
                  <label className="text-sm font-medium">Second Value</label>
                  <Input
                    placeholder="Second value..."
                    value={value2}
                    onChange={(e) => setValue2(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        handleApply();
                      }
                    }}
                  />
                </>
              )}
            </div>
          )}
        </div>
        <DialogFooter>
          <Button onClick={handleClear} variant="outline" size="sm">
            Clear
          </Button>
          <Button onClick={handleApply} size="sm">
            Apply Filter
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function EditableDataGrid({
  result,
  onGeographicCellClick,
  onCommitChanges,
  tableName,
  primaryKeyColumns,
}: EditableDataGridProps) {
  const addTableTab = useQueryStore((state) => state.addTableTab);

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
  const [editingCell, setEditingCell] = useState<{ rowIndex: number; columnName: string } | null>(null);
  const [editValue, setEditValue] = useState<string>("");
  const [editingCellRect, setEditingCellRect] = useState<DOMRect | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const cellRefs = useRef<Map<string, HTMLDivElement>>(new Map());
  const [selectedRows, setSelectedRows] = useState<Set<number>>(new Set());
  const clickTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const saveEditRef = useRef<(() => void) | null>(null);
  const tableContainerRef = useRef<HTMLDivElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);

  // Column filters
  const [columnFilters, setColumnFilters] = useState<ColumnFilter[]>([]);

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

  // Clear click timeout when editing state changes (prevents timeout from firing after edit starts)
  useEffect(() => {
    if (editingCell && clickTimeoutRef.current) {
      clearTimeout(clickTimeoutRef.current);
      clickTimeoutRef.current = null;
    }
  }, [editingCell]);

  // Clear editing state when pagination changes
  useEffect(() => {
    if (editingCell) {
      cancelEditing();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pagination.pageIndex, pagination.pageSize]);

  // Close editing on scroll
  useEffect(() => {
    const scrollContainer = scrollContainerRef.current;
    if (!scrollContainer || !editingCell) return;

    const handleScroll = () => {
      saveEditRef.current?.();
    };

    scrollContainer.addEventListener('scroll', handleScroll);
    return () => scrollContainer.removeEventListener('scroll', handleScroll);
  }, [editingCell]);

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

  // Apply column filters to a row
  const rowMatchesFilters = useCallback((row: Record<string, any>) => {
    return columnFilters.every((filter) => {
      const cellValue = row[filter.column];
      const filterValue = filter.value?.toLowerCase();

      // Handle empty/null checks
      if (filter.operator === "isEmpty") {
        return cellValue === null || cellValue === undefined || cellValue === "";
      }
      if (filter.operator === "isNotEmpty") {
        return cellValue !== null && cellValue !== undefined && cellValue !== "";
      }

      // If cell is null/undefined, skip other comparisons
      if (cellValue === null || cellValue === undefined) {
        return false;
      }

      const cellValueStr = String(cellValue).toLowerCase();
      const cellValueNum = Number(cellValue);

      switch (filter.operator) {
        case "equals":
          return cellValueStr === filterValue;
        case "contains":
          return cellValueStr.includes(filterValue);
        case "startsWith":
          return cellValueStr.startsWith(filterValue);
        case "endsWith":
          return cellValueStr.endsWith(filterValue);
        case "greaterThan":
          return !isNaN(cellValueNum) && cellValueNum > Number(filter.value);
        case "lessThan":
          return !isNaN(cellValueNum) && cellValueNum < Number(filter.value);
        case "between":
          if (filter.value2) {
            return !isNaN(cellValueNum) &&
                   cellValueNum >= Number(filter.value) &&
                   cellValueNum <= Number(filter.value2);
          }
          return false;
        default:
          return true;
      }
    });
  }, [columnFilters]);

  // Combine original data with inserts, apply deletes and filters
  const displayData = useMemo(() => {
    const data = [...result.rows, ...changes.inserts.map((insert) => insert.rowData)];
    const deletedFiltered = data.filter((_, idx) => !changes.deletes.has(idx));

    if (columnFilters.length === 0) {
      return deletedFiltered;
    }

    return deletedFiltered.filter(rowMatchesFilters);
  }, [result.rows, changes.inserts, changes.deletes, columnFilters, rowMatchesFilters]);

  // Map display indices to original indices (handles deleted rows causing index shifts)
  const rowIndexMap = useMemo(() => {
    const map = new Map<number, number>(); // displayIndex -> originalIndex
    let displayIndex = 0;

    // Map original rows
    for (let i = 0; i < result.rows.length; i++) {
      if (!changes.deletes.has(i)) {
        map.set(displayIndex, i);
        displayIndex++;
      }
    }

    // Map inserted rows
    for (let i = 0; i < changes.inserts.length; i++) {
      const originalIndex = result.rows.length + i;
      if (!changes.deletes.has(originalIndex)) {
        map.set(displayIndex, originalIndex);
        displayIndex++;
      }
    }

    return map;
  }, [result.rows.length, changes.deletes, changes.inserts.length]);

  // Convert display index to original index
  const getOriginalIndex = (displayIndex: number): number => {
    return rowIndexMap.get(displayIndex) ?? displayIndex;
  };

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
  const startEditing = useCallback((rowIndex: number, columnName: string, currentValue: any) => {
    const cellKey = `${rowIndex}-${columnName}`;
    const cellElement = cellRefs.current.get(cellKey);

    if (cellElement) {
      const rect = cellElement.getBoundingClientRect();
      const containerRect = tableContainerRef.current?.getBoundingClientRect();

      if (containerRect) {
        // Calculate position relative to the table container
        setEditingCellRect({
          top: rect.top - containerRect.top,
          left: rect.left - containerRect.left,
          width: rect.width,
          height: rect.height,
        } as DOMRect);
      }
    }

    setEditingCell({ rowIndex, columnName });
    setEditValue(currentValue === null ? "" : String(currentValue));
  }, []);

  // Cancel editing
  const cancelEditing = useCallback(() => {
    setEditingCell(null);
    setEditValue("");
  }, []);

  // Save cell edit (auto-save on blur)
  const saveEdit = useCallback(() => {
    if (!editingCell) return;

    const { rowIndex, columnName } = editingCell;

    // Get old value - handle both original rows and inserted rows
    let oldValue: any;
    if (rowIndex < result.rows.length) {
      // Original row
      oldValue = result.rows[rowIndex]?.[columnName];
    } else {
      // Inserted row
      const insertIndex = rowIndex - result.rows.length;
      oldValue = changes.inserts[insertIndex]?.rowData[columnName];
    }

    // Parse new value, handling type conversions
    let newValue: any = editValue === "" ? null : editValue;

    // Handle type matching: if old value is a number and new value can be parsed as number
    if (typeof oldValue === "number" && newValue !== null && !isNaN(Number(newValue))) {
      newValue = Number(newValue);
    }
    // Handle boolean matching
    else if (typeof oldValue === "boolean" && (newValue === "true" || newValue === "false")) {
      newValue = newValue === "true";
    }

    const key = getChangeKey(rowIndex, columnName);

    // If value changed from original, add/update edit
    if (oldValue !== newValue) {
      const newEdits = new Map(changes.edits);
      newEdits.set(key, { rowIndex, columnName, oldValue, newValue });
      setChanges({ ...changes, edits: newEdits });
    }
    // If value equals original, remove any existing edit
    else if (changes.edits.has(key)) {
      const newEdits = new Map(changes.edits);
      newEdits.delete(key);
      setChanges({ ...changes, edits: newEdits });
    }

    cancelEditing();
  }, [editingCell, editValue, result.rows, changes, cancelEditing]);

  // Keep ref updated with latest saveEdit function to avoid stale closures in event handlers
  // This intentionally runs on every render to capture the latest closure
  // eslint-disable-next-line react-hooks/exhaustive-deps
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

  // Filter management functions
  const addOrUpdateFilter = useCallback((filter: ColumnFilter) => {
    setColumnFilters((prev) => {
      const existingIndex = prev.findIndex((f) => f.column === filter.column);
      if (existingIndex >= 0) {
        const updated = [...prev];
        updated[existingIndex] = filter;
        return updated;
      }
      return [...prev, filter];
    });
  }, []);

  const removeFilter = useCallback((columnName: string) => {
    setColumnFilters((prev) => prev.filter((f) => f.column !== columnName));
  }, []);

  const clearAllFilters = useCallback(() => {
    setColumnFilters([]);
  }, []);

  const getColumnFilter = useCallback((columnName: string) => {
    return columnFilters.find((f) => f.column === columnName);
  }, [columnFilters]);

  // Calculate totals
  const totalChanges = changes.edits.size + changes.deletes.size + changes.inserts.length;
  const hasChanges = totalChanges > 0;

  // Helper function to get display value (edited or original)
  const getDisplayValue = useCallback((value: any, rowIndex: number, columnName: string) => {
    const changeKey = getChangeKey(rowIndex, columnName);
    const editedValue = changes.edits.get(changeKey);
    return editedValue ? editedValue.newValue : value;
  }, [changes.edits]);

  // Helper function to render cell values based on type
  const renderCellValue = useCallback((value: any, columnName: string, rowIndex: number) => {
    const displayValue = getDisplayValue(value, rowIndex, columnName);

    // Get FK metadata for this column
    const columnMeta = result.column_metadata?.find((meta) => meta.name === columnName);
    const hasForeignKey = columnMeta?.foreign_key;

    // Handle NULL
    if (displayValue === null || displayValue === undefined) {
      return (
        <div className="flex items-center gap-2">
          <span className="text-muted-foreground italic whitespace-nowrap">NULL</span>
          {hasForeignKey && <span className="w-4" />} {/* Spacer for alignment */}
        </div>
      );
    }

    // Wrapper to add FK icon if column is a foreign key
    const withForeignKeyIcon = (content: React.ReactNode) => {
      if (!hasForeignKey) return content;

      return (
        <div className="flex items-center gap-2 group/fk">
          {content}
          <button
            onClick={(e) => {
              e.stopPropagation();
              if (columnMeta?.foreign_key) {
                addTableTab(
                  columnMeta.foreign_key.referenced_table,
                  {
                    columnName: columnMeta.foreign_key.referenced_column,
                    value: displayValue,
                  }
                );
              }
            }}
            className="opacity-0 group-hover/fk:opacity-100 transition-opacity hover:text-blue-600 dark:hover:text-blue-400"
            title={`Go to ${columnMeta.foreign_key?.referenced_table}.${columnMeta.foreign_key?.referenced_column} = ${displayValue}`}
          >
            <ExternalLink className="h-3 w-3" />
          </button>
        </div>
      );
    };

    // Handle boolean
    if (typeof displayValue === "boolean") {
      return withForeignKeyIcon(
        <span className="font-mono px-2 py-0.5 rounded bg-blue-500/10 text-blue-700 dark:text-blue-400 whitespace-nowrap">
          {displayValue.toString()}
        </span>
      );
    }

    // Handle number
    if (typeof displayValue === "number") {
      return withForeignKeyIcon(
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
        return withForeignKeyIcon(
          <span className="font-mono text-green-700 dark:text-green-400 whitespace-nowrap" title={displayValue}>
            {displayValue}
          </span>
        );
      }

      // Check for hex binary data (BYTEA, BLOB)
      if (displayValue.startsWith("0x") || displayValue.startsWith("\\x")) {
        const displayText =
          displayValue.length > 50 ? displayValue.substring(0, 50) + "..." : displayValue;
        return withForeignKeyIcon(
          <span className="font-mono text-orange-700 dark:text-orange-400 text-xs whitespace-nowrap" title={displayValue}>
            {displayText}
          </span>
        );
      }

      // Check for PostgreSQL bit strings (b'10101')
      if (/^b'[01]+'$/.test(displayValue)) {
        return withForeignKeyIcon(
          <span className="font-mono text-orange-600 dark:text-orange-300 text-xs whitespace-nowrap" title={displayValue}>
            {displayValue}
          </span>
        );
      }

      // Check for NaN/Infinity special float values
      if (displayValue === "NaN" || displayValue === "Infinity" || displayValue === "-Infinity") {
        return withForeignKeyIcon(
          <span className="font-mono text-yellow-700 dark:text-yellow-400 italic whitespace-nowrap" title={displayValue}>
            {displayValue}
          </span>
        );
      }

      // Check for IP address / CIDR notation (INET, CIDR types)
      const ipPattern = /^(\d{1,3}\.){3}\d{1,3}(\/\d{1,2})?$|^([0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}(\/\d{1,3})?$/;
      if (ipPattern.test(displayValue)) {
        return withForeignKeyIcon(
          <span className="font-mono text-teal-700 dark:text-teal-400 text-sm whitespace-nowrap" title={displayValue}>
            {displayValue}
          </span>
        );
      }

      // Check for MAC address pattern
      const macPattern = /^([0-9a-fA-F]{2}:){5}[0-9a-fA-F]{2}$/;
      if (macPattern.test(displayValue)) {
        return withForeignKeyIcon(
          <span className="font-mono text-teal-600 dark:text-teal-300 text-sm whitespace-nowrap" title={displayValue}>
            {displayValue}
          </span>
        );
      }

      // Check for PostgreSQL range types [x,y) or (x,y] etc.
      const rangePattern = /^[\[\(].+,.+[\]\)]$/;
      if (rangePattern.test(displayValue)) {
        return withForeignKeyIcon(
          <span className="font-mono text-violet-700 dark:text-violet-400 text-sm whitespace-nowrap" title={displayValue}>
            {displayValue}
          </span>
        );
      }

      // Check for PostgreSQL interval format (includes months/days or HH:MM:SS)
      const intervalPattern = /^\d+\s+months?\s+\d+\s+days?\s+\d{2}:\d{2}:\d{2}|^\d{2}:\d{2}:\d{2}\.\d+$/;
      if (intervalPattern.test(displayValue)) {
        return withForeignKeyIcon(
          <span className="font-mono text-amber-700 dark:text-amber-400 text-sm whitespace-nowrap" title={displayValue}>
            {displayValue}
          </span>
        );
      }

      // Check for UUID pattern
      const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
      if (uuidPattern.test(displayValue)) {
        return withForeignKeyIcon(
          <span className="font-mono text-cyan-700 dark:text-cyan-400 text-sm whitespace-nowrap" title={displayValue}>
            {displayValue}
          </span>
        );
      }

      // Regular string - single line with truncation
      return withForeignKeyIcon(
        <span className="block truncate max-w-[300px] whitespace-nowrap" title={displayValue}>
          {displayValue}
        </span>
      );
    }

    // Handle object (like JSON) or array
    if (typeof displayValue === "object") {
      const isArray = Array.isArray(displayValue);
      const jsonStr = JSON.stringify(displayValue);
      const displayStr = jsonStr.length > 100 ? jsonStr.substring(0, 100) + "..." : jsonStr;

      // Arrays get a different color (pink/rose) to distinguish from objects (indigo)
      const colorClass = isArray
        ? "text-rose-700 dark:text-rose-400"
        : "text-indigo-700 dark:text-indigo-400";

      return withForeignKeyIcon(
        <span
          className={`font-mono text-sm ${colorClass} block truncate max-w-[300px] whitespace-nowrap`}
          title={jsonStr}
        >
          {displayStr}
        </span>
      );
    }

    // Fallback
    return withForeignKeyIcon(
      <span className="truncate block max-w-[300px] whitespace-nowrap" title={String(displayValue)}>
        {String(displayValue)}
      </span>
    );
  }, [getDisplayValue, onGeographicCellClick, result.column_metadata, addTableTab]);

  // Create columns from query result
  const columns = useMemo<ColumnDef<Record<string, any>>[]>(() => {
    const dataColumns: ColumnDef<Record<string, any>>[] = [
      // Checkbox column for row selection
      {
        id: "select",
        header: ({ table }) => {
          const pageRows = table.getRowModel().rows;
          const pageRowIndices = pageRows.map((row) => getOriginalIndex(row.index));
          const allSelected = pageRowIndices.length > 0 && pageRowIndices.every((idx) => selectedRows.has(idx));

          const toggleAll = () => {
            const newSelection = new Set(selectedRows);
            if (allSelected) {
              pageRowIndices.forEach((idx) => newSelection.delete(idx));
            } else {
              pageRowIndices.forEach((idx) => newSelection.add(idx));
            }
            setSelectedRows(newSelection);
          };

          return (
            <div className="flex items-center justify-center py-2">
              <input
                type="checkbox"
                checked={allSelected}
                onChange={toggleAll}
                className="h-4 w-4 rounded border-gray-300"
              />
            </div>
          );
        },
        cell: ({ row }) => {
          const originalIndex = getOriginalIndex(row.index);
          const isDeleted = isRowDeleted(originalIndex);
          return (
            <div className="flex items-center justify-center py-2">
              <input
                type="checkbox"
                checked={selectedRows.has(originalIndex)}
                onChange={() => toggleRowSelection(originalIndex)}
                disabled={isDeleted}
                className="h-4 w-4 rounded border-gray-300"
              />
            </div>
          );
        },
      },
      // Data columns
      ...result.columns.map((columnName) => {
        const columnMeta = result.column_metadata?.find((m) => m.name === columnName);

        return {
          accessorKey: columnName,
          header: () => (
            <div className="group flex items-center gap-2">
              <span>{columnName}</span>
              <FilterPopover
                columnName={columnName}
                dataType={columnMeta?.data_type || "TEXT"}
                currentFilter={getColumnFilter(columnName)}
                onApplyFilter={addOrUpdateFilter}
                onRemoveFilter={() => removeFilter(columnName)}
              />
            </div>
          ),
          cell: ({ getValue, row }: any) => {
          const originalIndex = getOriginalIndex(row.index);
          const value = getValue();
          const isEdited = isCellEdited(originalIndex, columnName);

          const isEditing = editingCell?.rowIndex === originalIndex && editingCell?.columnName === columnName;

          return (
            <div
              className={cn(
                "group relative py-2 px-3 overflow-hidden cursor-cell hover:bg-muted/50 transition-colors",
                isEdited && "bg-blue-50 dark:bg-blue-950/30",
                isRowDeleted(originalIndex) && "opacity-50 cursor-not-allowed",
                isRowInserted(originalIndex) && "bg-green-100 dark:bg-green-900/40",
                isEditing && "cursor-text bg-blue-100 dark:bg-blue-900/50"
              )}
              onClick={() => {
                // Don't trigger edit mode if already editing
                if (isEditing || isRowDeleted(originalIndex)) return;

                // Clear any existing timeout
                if (clickTimeoutRef.current) {
                  clearTimeout(clickTimeoutRef.current);
                  clickTimeoutRef.current = null;
                }

                // Set a timeout for single click - will be cleared if double-click happens
                clickTimeoutRef.current = setTimeout(() => {
                  startEditing(originalIndex, columnName, value);
                  clickTimeoutRef.current = null;
                }, 250);
              }}
              onDoubleClick={(e) => {
                if (!isRowDeleted(originalIndex) && !isEditing) {
                  e.stopPropagation();

                  // Clear single click timeout
                  if (clickTimeoutRef.current) {
                    clearTimeout(clickTimeoutRef.current);
                    clickTimeoutRef.current = null;
                  }

                  startEditing(originalIndex, columnName, value);
                }
              }}
              title={isRowDeleted(originalIndex) ? "Row is deleted" : isEditing ? "" : "Click to edit"}
              ref={(el) => {
                const cellKey = `${originalIndex}-${columnName}`;
                if (el) {
                  cellRefs.current.set(cellKey, el);
                } else {
                  cellRefs.current.delete(cellKey);
                }
              }}
            >
              <div className="overflow-x-auto whitespace-nowrap">
                {/* Always show the cell value - editing happens in floating overlay */}
                {renderCellValue(value, columnName, originalIndex)}
              </div>
            </div>
          );
        },
        };
      }),
    ];

    return dataColumns;
    // Note: Editing state (editingCell, editValue, etc.) completely excluded from deps
    // Editing now happens in a floating overlay outside the table, so columns never need to recreate
  }, [result.columns, selectedRows, displayData.length, rowIndexMap, result.rows.length, renderCellValue, getOriginalIndex, getColumnFilter, addOrUpdateFilter, removeFilter, result.column_metadata]);

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

  // Get column metadata for the currently editing cell
  const editingColumnMetadata = editingCell
    ? result.column_metadata?.find((meta) => meta.name === editingCell.columnName)
    : null;

  const isBooleanColumn = editingColumnMetadata?.data_type === "BOOL" ||
                          editingColumnMetadata?.data_type === "BOOLEAN" ||
                          editingColumnMetadata?.data_type === "TINYINT(1)";

  // Enum detection: Check if enum_values are populated (works for both PostgreSQL custom enums and MySQL ENUM/SET)
  // PostgreSQL enums have the custom type name as data_type, not "ENUM"
  // MySQL has "ENUM" or "SET" as data_type
  const isEnumColumn = editingColumnMetadata?.enum_values &&
                       editingColumnMetadata.enum_values.length > 0;

  return (
    <div className="flex flex-col h-full relative" ref={tableContainerRef}>
      {/* Floating Edit Input Overlay */}
      {editingCell && editingCellRect && (
        <div
          className="absolute z-50 pointer-events-auto"
          style={{
            top: `${editingCellRect.top}px`,
            left: `${editingCellRect.left}px`,
            width: `${editingCellRect.width}px`,
            height: `${editingCellRect.height}px`,
          }}
        >
          {isBooleanColumn ? (
            <Select
              value={editValue === "true" || editValue === "1" ? "true" : editValue === "false" || editValue === "0" ? "false" : "null"}
              onValueChange={(value) => {
                setEditValue(value);
                // Auto-save on selection
                setTimeout(() => saveEdit(), 0);
              }}
              onOpenChange={(open) => {
                if (!open) {
                  saveEdit();
                }
              }}
            >
              <SelectTrigger className="h-full text-sm w-full border-2 border-blue-500" autoFocus>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="true">true</SelectItem>
                <SelectItem value="false">false</SelectItem>
                <SelectItem value="null">NULL</SelectItem>
              </SelectContent>
            </Select>
          ) : isEnumColumn ? (
            <Select
              value={editValue || editingColumnMetadata?.enum_values?.[0] || "null"}
              onValueChange={(value) => {
                setEditValue(value);
                setTimeout(() => saveEdit(), 0);
              }}
              onOpenChange={(open) => {
                if (!open) saveEdit();
              }}
            >
              <SelectTrigger className="h-full text-sm w-full border-2 border-blue-500" autoFocus>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {editingColumnMetadata?.enum_values?.map((enumValue) => (
                  <SelectItem key={enumValue} value={enumValue}>
                    {enumValue}
                  </SelectItem>
                ))}
                <SelectItem value="null">NULL</SelectItem>
              </SelectContent>
            </Select>
          ) : (
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
              onClick={(e) => e.stopPropagation()}
              onBlur={saveEdit}
              className="h-full text-sm w-full border-2 border-blue-500"
              autoFocus
            />
          )}
        </div>
      )}

      {/* Header with results info */}
      <div className="flex items-center justify-between px-4 py-2 border-b bg-card">
        <div className="flex items-center gap-3">
          <span className="text-sm font-semibold">
            {displayData.length} / {result.row_count} row{result.row_count !== 1 ? "s" : ""}
          </span>
          <span className="text-sm text-muted-foreground">in {result.execution_time_ms}ms</span>
          {columnFilters.length > 0 && (
            <Button
              variant="outline"
              size="sm"
              onClick={clearAllFilters}
              className="h-7 text-xs gap-1.5"
            >
              <FilterX className="h-3.5 w-3.5" />
              Clear {columnFilters.length} filter{columnFilters.length !== 1 ? 's' : ''}
            </Button>
          )}
          {hasChanges && (
            <div className="flex items-center gap-2 ml-4">
              {changes.edits.size > 0 && (
                <Badge variant="secondary" className="h-5 min-w-5 rounded-full px-2 font-mono tabular-nums text-xs bg-blue-500 text-white dark:bg-blue-600">
                  {changes.edits.size}
                </Badge>
              )}
              {changes.deletes.size > 0 && (
                <Badge variant="destructive" className="h-5 min-w-5 rounded-full px-2 font-mono tabular-nums text-xs">
                  {changes.deletes.size}
                </Badge>
              )}
              {changes.inserts.length > 0 && (
                <Badge className="h-5 min-w-5 rounded-full px-2 font-mono tabular-nums text-xs bg-green-600 hover:bg-green-700 dark:bg-green-600 dark:hover:bg-green-700">
                  {changes.inserts.length}
                </Badge>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Table */}
      <div className="flex-1 overflow-auto relative data-grid-mobile" ref={scrollContainerRef}>
        <Table>
          <TableHeader className="shadow-sm">
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
                const originalIndex = getOriginalIndex(row.index);
                const isDeleted = isRowDeleted(originalIndex);
                const isNew = isRowInserted(originalIndex);
                const isSelected = selectedRows.has(originalIndex);

                return (
                  <TableRow
                    key={row.id}
                    data-state={isSelected ? "selected" : undefined}
                    className={cn(
                      "transition-colors duration-150",
                      "hover:bg-muted/50",
                      isSelected && "bg-muted/50",
                      isDeleted && "line-through opacity-60 bg-red-50 dark:bg-red-950/20",
                      isNew && "bg-green-100 dark:bg-green-900/40"
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
      <div className={`flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-2 px-4 py-2 border-t bg-card/50 ${displayData.length === 0 ? 'safe-area-bottom' : ''}`}>
        <div className="flex items-center gap-2">
          <Button size="sm" variant="outline" onClick={addRow} className="h-9 md:h-8">
            <Plus className="h-4 w-4 mr-2" />
            <span className="hidden sm:inline">Add Row</span>
            <span className="sm:hidden">Add</span>
          </Button>
          {selectedRows.size > 0 && (
            <Button
              size="sm"
              variant="outline"
              onClick={deleteSelectedRows}
              className="h-9 md:h-8 text-destructive hover:text-destructive/80"
            >
              <Trash2 className="h-4 w-4 mr-2" />
              <span className="hidden sm:inline">Delete Selected</span>
              <span className="sm:hidden">Delete</span>
              ({selectedRows.size})
            </Button>
          )}
        </div>

        <div className="flex items-center gap-2 justify-end">
          {hasChanges && (
            <>
              <Button size="sm" variant="outline" onClick={resetChanges} className="h-9 md:h-8">
                <RotateCcw className="h-4 w-4 mr-2" />
                Reset
              </Button>
              <Button
                size="sm"
                onClick={handleCommit}
                className="h-9 md:h-8"
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
        <div className="flex flex-col md:flex-row items-center justify-between gap-2 px-4 py-3 border-t bg-card safe-area-bottom">
          {/* Page info - hidden on small mobile */}
          <div className="hidden sm:flex items-center gap-2">
            <p className="text-sm text-muted-foreground">
              Page {table.getState().pagination.pageIndex + 1} of {table.getPageCount()}
            </p>
          </div>

          <div className="flex items-center gap-2 md:gap-6 w-full md:w-auto justify-between md:justify-end">
            {/* Rows per page - compact on mobile */}
            <div className="flex items-center gap-2">
              <p className="text-xs md:text-sm text-muted-foreground whitespace-nowrap hidden sm:block">Rows per page</p>
              <Select
                value={`${table.getState().pagination.pageSize}`}
                onValueChange={(value) => {
                  table.setPageSize(Number(value));
                }}
              >
                <SelectTrigger className="h-9 w-[70px] md:h-8">
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

            {/* Navigation buttons - touch-friendly */}
            <div className="flex items-center gap-1">
              <Button
                variant="outline"
                size="icon"
                className="h-9 w-9 md:h-8 md:w-8"
                onClick={() => table.firstPage()}
                disabled={!table.getCanPreviousPage()}
              >
                <ChevronsLeft className="h-4 w-4" />
              </Button>
              <Button
                variant="outline"
                size="icon"
                className="h-9 w-9 md:h-8 md:w-8"
                onClick={() => table.previousPage()}
                disabled={!table.getCanPreviousPage()}
              >
                <ChevronLeft className="h-4 w-4" />
              </Button>
              <Button
                variant="outline"
                size="icon"
                className="h-9 w-9 md:h-8 md:w-8"
                onClick={() => table.nextPage()}
                disabled={!table.getCanNextPage()}
              >
                <ChevronRight className="h-4 w-4" />
              </Button>
              <Button
                variant="outline"
                size="icon"
                className="h-9 w-9 md:h-8 md:w-8"
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
