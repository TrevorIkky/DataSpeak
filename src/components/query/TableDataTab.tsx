import { useEffect, useMemo } from "react";
import {
  useReactTable,
  getCoreRowModel,
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
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight, Loader2, Table as TableIcon, Info, Network } from "lucide-react";
import { useQueryStore } from "@/stores/queryStore";
import { useConnectionStore } from "@/stores/connectionStore";
import { useSchemaStore } from "@/stores/schemaStore";
import { TableProperties } from "./TableProperties";
import { ERDViewer } from "@/components/erd/ERDViewer";
import type { TableTab } from "@/types/query.types";

interface TableDataTabProps {
  tab: TableTab;
}

export function TableDataTab({ tab }: TableDataTabProps) {
  const { loadTableData, tabs, updateTab } = useQueryStore();
  const { activeConnection } = useConnectionStore();
  const { schema } = useSchemaStore();

  // Load initial data
  useEffect(() => {
    if (activeConnection && !tab.result && !tab.isLoading) {
      loadTableData(tab.id, activeConnection.id);
    }
  }, [activeConnection, tab.id, tab.result, tab.isLoading, loadTableData]);

  // Reload data when pagination changes
  useEffect(() => {
    if (activeConnection && tab.result) {
      loadTableData(tab.id, activeConnection.id);
    }
  }, [tab.pagination.pageIndex, tab.pagination.pageSize]);

  // Create columns from query result
  const columns = useMemo<ColumnDef<Record<string, any>>[]>(() => {
    if (!tab.result) return [];

    return tab.result.columns.map((columnName) => ({
      accessorKey: columnName,
      header: columnName,
      cell: ({ getValue }) => {
        const value = getValue();
        if (value === null || value === undefined) {
          return <span className="text-muted-foreground italic">NULL</span>;
        }
        if (typeof value === "boolean") {
          return <span className="font-mono">{value.toString()}</span>;
        }
        if (typeof value === "number") {
          return <span className="font-mono">{value.toLocaleString()}</span>;
        }
        return <span className="truncate block max-w-xs">{String(value)}</span>;
      },
    }));
  }, [tab.result]);

  const table = useReactTable({
    data: tab.result?.rows || [],
    columns,
    getCoreRowModel: getCoreRowModel(),
    manualPagination: true, // Server-side pagination
    pageCount: -1, // Unknown total pages (we'll handle this manually)
  });

  const handlePageSizeChange = (newSize: number) => {
    // Update the tab's pagination in the store
    const updatedTabs = tabs.map((t) => {
      if (t.id === tab.id && t.type === 'table') {
        return {
          ...t,
          pagination: {
            pageIndex: 0, // Reset to first page
            pageSize: newSize,
          },
        };
      }
      return t;
    });

    // This will trigger the useEffect to reload data
    useQueryStore.setState({ tabs: updatedTabs });
  };

  const handlePageChange = (newPageIndex: number) => {
    const updatedTabs = tabs.map((t) => {
      if (t.id === tab.id && t.type === 'table') {
        return {
          ...t,
          pagination: {
            ...t.pagination,
            pageIndex: newPageIndex,
          },
        };
      }
      return t;
    });

    useQueryStore.setState({ tabs: updatedTabs });
  };

  const handleViewModeChange = (mode: 'data' | 'properties' | 'erd') => {
    updateTab(tab.id, { viewMode: mode });
  };

  // Find the current table in schema
  const currentTable = schema?.tables.find(t => t.name === tab.tableName);

  if (tab.isLoading) {
    return (
      <div className="flex flex-col items-center justify-center h-full">
        <Loader2 className="h-8 w-8 animate-spin text-primary mb-2" />
        <p className="text-sm text-muted-foreground">Loading table data...</p>
      </div>
    );
  }

  if (tab.error) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-8">
        <p className="text-sm text-destructive">{tab.error}</p>
      </div>
    );
  }

  if (!tab.result) {
    return (
      <div className="flex flex-col items-center justify-center h-full">
        <p className="text-sm text-muted-foreground">No data loaded</p>
      </div>
    );
  }

  const hasNextPage = tab.result.rows.length === tab.pagination.pageSize;
  const hasPreviousPage = tab.pagination.pageIndex > 0;

  return (
    <div className="flex flex-col h-full">
      {/* Conditional View Rendering */}
      {tab.viewMode === 'properties' ? (
        <>
          {/* Header for Properties View */}
          <div className="flex items-center justify-between px-4 py-2 border-b bg-card">
            <div className="text-sm">
              <span className="font-semibold">{tab.tableName}</span>
            </div>
            <Tabs value={tab.viewMode} onValueChange={(value) => handleViewModeChange(value as 'data' | 'properties' | 'erd')}>
              <TabsList className="h-8">
                <TabsTrigger value="data" className="flex items-center gap-1.5 text-xs h-7">
                  <TableIcon className="h-3.5 w-3.5" />
                  Data
                </TabsTrigger>
                <TabsTrigger value="properties" className="flex items-center gap-1.5 text-xs h-7">
                  <Info className="h-3.5 w-3.5" />
                  Properties
                </TabsTrigger>
                <TabsTrigger value="erd" className="flex items-center gap-1.5 text-xs h-7">
                  <Network className="h-3.5 w-3.5" />
                  ERD
                </TabsTrigger>
              </TabsList>
            </Tabs>
          </div>
          <TableProperties table={currentTable} tableName={tab.tableName} />
        </>
      ) : tab.viewMode === 'erd' ? (
        <>
          {/* Header for ERD View */}
          <div className="flex items-center justify-between px-4 py-2 border-b bg-card">
            <div className="text-sm">
              <span className="font-semibold">{tab.tableName}</span>
            </div>
            <Tabs value={tab.viewMode} onValueChange={(value) => handleViewModeChange(value as 'data' | 'properties' | 'erd')}>
              <TabsList className="h-8">
                <TabsTrigger value="data" className="flex items-center gap-1.5 text-xs h-7">
                  <TableIcon className="h-3.5 w-3.5" />
                  Data
                </TabsTrigger>
                <TabsTrigger value="properties" className="flex items-center gap-1.5 text-xs h-7">
                  <Info className="h-3.5 w-3.5" />
                  Properties
                </TabsTrigger>
                <TabsTrigger value="erd" className="flex items-center gap-1.5 text-xs h-7">
                  <Network className="h-3.5 w-3.5" />
                  ERD
                </TabsTrigger>
              </TabsList>
            </Tabs>
          </div>
          <ERDViewer tables={schema?.tables || []} focusTableName={tab.tableName} />
        </>
      ) : (
        // Data View (default)
        <div className="flex flex-col h-full">
      {/* Results Info with View Mode Selector */}
      <div className="flex items-center justify-between px-4 py-2 border-b bg-card">
        <div className="text-sm">
          <span className="font-semibold">{tab.tableName}</span>
          <span className="text-muted-foreground ml-2">
            Showing {tab.result.rows.length} rows
          </span>
          {tab.result.execution_time_ms && (
            <span className="text-muted-foreground ml-2">
              in {tab.result.execution_time_ms}ms
            </span>
          )}
        </div>
        <Tabs value={tab.viewMode} onValueChange={(value) => handleViewModeChange(value as 'data' | 'properties' | 'erd')}>
          <TabsList className="h-8">
            <TabsTrigger value="data" className="flex items-center gap-1.5 text-xs h-7">
              <TableIcon className="h-3.5 w-3.5" />
              Data
            </TabsTrigger>
            <TabsTrigger value="properties" className="flex items-center gap-1.5 text-xs h-7">
              <Info className="h-3.5 w-3.5" />
              Properties
            </TabsTrigger>
            <TabsTrigger value="erd" className="flex items-center gap-1.5 text-xs h-7">
              <Network className="h-3.5 w-3.5" />
              ERD
            </TabsTrigger>
          </TabsList>
        </Tabs>
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

      {/* Server-Side Pagination */}
      <div className="flex items-center justify-between px-4 py-3 border-t bg-card">
        <div className="flex items-center gap-2">
          <p className="text-sm text-muted-foreground">
            Page {tab.pagination.pageIndex + 1}
          </p>
        </div>

        <div className="flex items-center gap-6">
          <div className="flex items-center gap-2">
            <p className="text-sm text-muted-foreground whitespace-nowrap">
              Rows per page
            </p>
            <Select
              value={`${tab.pagination.pageSize}`}
              onValueChange={(value) => handlePageSizeChange(Number(value))}
            >
              <SelectTrigger className="h-8 w-[70px]">
                <SelectValue placeholder={tab.pagination.pageSize} />
              </SelectTrigger>
              <SelectContent side="top">
                {[10, 20, 50, 100, 200, 500].map((pageSize) => (
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
              onClick={() => handlePageChange(0)}
              disabled={!hasPreviousPage || tab.isLoading}
            >
              <ChevronsLeft className="h-4 w-4" />
            </Button>
            <Button
              variant="outline"
              size="icon"
              className="h-8 w-8"
              onClick={() => handlePageChange(tab.pagination.pageIndex - 1)}
              disabled={!hasPreviousPage || tab.isLoading}
            >
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <Button
              variant="outline"
              size="icon"
              className="h-8 w-8"
              onClick={() => handlePageChange(tab.pagination.pageIndex + 1)}
              disabled={!hasNextPage || tab.isLoading}
            >
              <ChevronRight className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </div>
        </div>
      )}
    </div>
  );
}
