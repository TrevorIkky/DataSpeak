import { useEffect } from "react";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Loader2, Table as TableIcon, Info, Network } from "lucide-react";
import { useQueryStore } from "@/stores/queryStore";
import { useConnectionStore } from "@/stores/connectionStore";
import { useSchemaStore } from "@/stores/schemaStore";
import { TableProperties } from "./TableProperties";
import { ERDViewer } from "@/components/erd/ERDViewer";
import { EditableDataGrid } from "./EditableDataGrid";
import type { TableTab } from "@/types/query.types";
import type { DataGridChanges } from "@/types/datagrid.types";
import { commitDataChanges } from "@/api/datagrid";
import { toast } from "sonner";

interface TableDataTabProps {
  tab: TableTab;
}

export function TableDataTab({ tab }: TableDataTabProps) {
  const { loadTableData, updateTab } = useQueryStore();
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

  const handleViewModeChange = (mode: 'data' | 'properties' | 'erd') => {
    updateTab(tab.id, { viewMode: mode });
  };

  // Find the current table in schema
  const currentTable = schema?.tables.find(t => t.name === tab.tableName);

  // Extract primary key columns from schema
  const primaryKeyColumns = currentTable?.columns
    .filter(col => col.is_primary_key)
    .map(col => col.name) || [];

  // Handle commit changes
  const handleCommitChanges = async (changes: DataGridChanges, originalRows: Record<string, any>[]) => {
    if (!activeConnection || !currentTable) {
      toast.error("Cannot commit changes: No active connection or table schema");
      return;
    }

    if (primaryKeyColumns.length === 0) {
      toast.error("Cannot commit changes: Table has no primary key");
      return;
    }

    try {
      const result = await commitDataChanges(
        activeConnection.id,
        tab.tableName,
        primaryKeyColumns,
        changes,
        originalRows
      );

      toast.success(result.message);

      // Reload table data to show committed changes
      loadTableData(tab.id, activeConnection.id);
    } catch (error: any) {
      console.error("Failed to commit changes:", error);
      toast.error(`Failed to commit changes: ${error.message || error}`);
    }
  };

  // Enrich result with schema columns if result has no columns (empty table)
  const enrichedResult = tab.result && (!tab.result.columns || tab.result.columns.length === 0) && currentTable
    ? {
        ...tab.result,
        columns: currentTable.columns.map(col => col.name),
      }
    : tab.result;

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

  if (!enrichedResult) {
    return (
      <div className="flex flex-col items-center justify-center h-full">
        <p className="text-sm text-muted-foreground">No data loaded</p>
      </div>
    );
  }

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
        <>
          {/* View Mode Selector */}
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

          {/* Editable Data Grid - flex-1 to take remaining space */}
          <div className="flex-1 min-h-0 flex flex-col">
            <EditableDataGrid
              result={enrichedResult}
              tableName={tab.tableName}
              primaryKeyColumns={primaryKeyColumns}
              onCommitChanges={handleCommitChanges}
            />
          </div>
        </>
      )}
    </div>
  );
}
