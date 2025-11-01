import { useMemo, useState } from "react";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight, MapPin } from "lucide-react";
import type { QueryResult } from "@/types/query.types";
import { isWKTGeometry, parseWKT } from "@/lib/geoUtils";
import type { GeographicCell } from "@/types/geography.types";

interface DataGridProps {
  result: QueryResult;
  onGeographicCellClick?: (cell: GeographicCell) => void;
}

export function DataGrid({ result, onGeographicCellClick }: DataGridProps) {
  const [pagination, setPagination] = useState({
    pageIndex: 0,
    pageSize: 50,
  });

  // Helper function to render cell values based on type
  const renderCellValue = (value: any, columnName: string, rowIndex: number) => {
    // Handle NULL
    if (value === null || value === undefined) {
      return <span className="text-muted-foreground italic">NULL</span>;
    }

    // Handle boolean
    if (typeof value === "boolean") {
      return (
        <span className="font-mono px-2 py-0.5 rounded bg-blue-500/10 text-blue-700 dark:text-blue-400">
          {value.toString()}
        </span>
      );
    }

    // Handle number
    if (typeof value === "number") {
      return <span className="font-mono text-purple-700 dark:text-purple-400">{value.toLocaleString()}</span>;
    }

    // Handle string
    if (typeof value === "string") {
      // Check for date/timestamp patterns (ISO 8601 format)
      const datePattern = /^\d{4}-\d{2}-\d{2}$/;
      const timePattern = /^\d{2}:\d{2}:\d{2}/;
      const dateTimePattern = /^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}/;

      if (dateTimePattern.test(value)) {
        return (
          <span className="font-mono text-green-700 dark:text-green-400" title={value}>
            {value}
          </span>
        );
      }

      if (datePattern.test(value)) {
        return (
          <span className="font-mono text-green-700 dark:text-green-400" title={value}>
            {value}
          </span>
        );
      }

      if (timePattern.test(value)) {
        return (
          <span className="font-mono text-green-700 dark:text-green-400" title={value}>
            {value}
          </span>
        );
      }

      // Check for WKT geometry data (PostGIS/MySQL Spatial)
      if (isWKTGeometry(value)) {
        const geometry = parseWKT(value);
        const displayValue = value.length > 40 ? value.substring(0, 40) + "..." : value;

        return (
          <button
            onClick={() => {
              if (geometry && onGeographicCellClick) {
                onGeographicCellClick({
                  columnName,
                  rowIndex,
                  geometry,
                  rawValue: value,
                });
              }
            }}
            className="flex items-center gap-2 font-mono text-green-700 dark:text-green-400 text-sm hover:underline cursor-pointer"
            title={`Click to view on map\n\n${value}`}
          >
            <MapPin className="h-3 w-3 flex-shrink-0" />
            <span className="truncate">{displayValue}</span>
          </button>
        );
      }

      // Check for hex binary data
      if (value.startsWith("0x")) {
        const displayValue = value.length > 50 ? value.substring(0, 50) + "..." : value;
        return (
          <span className="font-mono text-orange-700 dark:text-orange-400 text-xs" title={value}>
            {displayValue}
          </span>
        );
      }

      // Check for UUID pattern
      const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
      if (uuidPattern.test(value)) {
        return (
          <span className="font-mono text-cyan-700 dark:text-cyan-400 text-sm" title={value}>
            {value}
          </span>
        );
      }

      // Check for unsupported type marker
      if (value.startsWith("<unsupported")) {
        return <span className="text-yellow-700 dark:text-yellow-400 italic text-sm">{value}</span>;
      }

      // Regular string - truncate if too long
      if (value.length > 200) {
        return (
          <span className="block max-w-xs truncate" title={value}>
            {value}
          </span>
        );
      }

      return <span className="block max-w-xs">{value}</span>;
    }

    // Handle object (like JSON)
    if (typeof value === "object") {
      const jsonStr = JSON.stringify(value, null, 2);
      const displayStr = jsonStr.length > 100 ? JSON.stringify(value) : jsonStr;
      return (
        <span className="font-mono text-sm text-indigo-700 dark:text-indigo-400 block max-w-xs truncate" title={jsonStr}>
          {displayStr}
        </span>
      );
    }

    // Fallback
    return <span className="truncate block max-w-xs">{String(value)}</span>;
  };

  // Create columns from query result
  const columns = useMemo<ColumnDef<Record<string, any>>[]>(() => {
    return result.columns.map((columnName) => ({
      accessorKey: columnName,
      header: columnName,
      cell: ({ getValue, row }) => renderCellValue(getValue(), columnName, row.index),
    }));
  }, [result.columns, onGeographicCellClick]);

  const table = useReactTable({
    data: result.rows,
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
      {/* Results Info */}
      <div className="flex items-center justify-between px-4 py-2 border-b bg-card">
        <div className="text-sm">
          <span className="font-semibold">{result.row_count}</span> row{result.row_count !== 1 ? "s" : ""} returned
          <span className="text-muted-foreground ml-2">
            in {result.execution_time_ms}ms
          </span>
        </div>
      </div>

      {/* Table */}
      <div className="flex-1 overflow-auto">
        <Table>
          <TableHeader className="sticky top-0 bg-card z-10">
            {table.getHeaderGroups().map((headerGroup) => (
              <TableRow key={headerGroup.id}>
                {headerGroup.headers.map((header) => (
                  <TableHead key={header.id} className="font-semibold">
                    {header.isPlaceholder
                      ? null
                      : flexRender(
                          header.column.columnDef.header,
                          header.getContext()
                        )}
                  </TableHead>
                ))}
              </TableRow>
            ))}
          </TableHeader>
          <TableBody>
            {table.getRowModel().rows?.length ? (
              table.getRowModel().rows.map((row) => (
                <TableRow
                  key={row.id}
                  data-state={row.getIsSelected() && "selected"}
                  className="hover:bg-muted/50"
                >
                  {row.getVisibleCells().map((cell) => (
                    <TableCell key={cell.id} className="py-2">
                      {flexRender(
                        cell.column.columnDef.cell,
                        cell.getContext()
                      )}
                    </TableCell>
                  ))}
                </TableRow>
              ))
            ) : (
              <TableRow>
                <TableCell
                  colSpan={columns.length}
                  className="h-24 text-center"
                >
                  No results.
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>

      {/* Pagination */}
      <div className="flex items-center justify-between px-4 py-3 border-t bg-card">
        <div className="flex items-center gap-2">
          <p className="text-sm text-muted-foreground">
            Page {table.getState().pagination.pageIndex + 1} of{" "}
            {table.getPageCount()}
          </p>
        </div>

        <div className="flex items-center gap-6">
          <div className="flex items-center gap-2">
            <p className="text-sm text-muted-foreground whitespace-nowrap">
              Rows per page
            </p>
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
    </div>
  );
}
