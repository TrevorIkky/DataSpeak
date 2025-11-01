import { useEffect, useState } from "react";
import { ChevronRight, ChevronDown, Table as TableIcon, Key, Link2, Loader2, Download, Upload } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { useSchemaStore } from "@/stores/schemaStore";
import { useConnectionStore } from "@/stores/connectionStore";
import { ExportDialog } from "@/components/export_import/ExportDialog";
import { ImportDialog } from "@/components/export_import/ImportDialog";
import type { Table } from "@/types/database.types";

interface TableItemProps {
  table: Table;
  onSelectTable: (table: Table) => void;
}

function TableItem({ table, onSelectTable }: TableItemProps) {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <div className="group">
        <CollapsibleTrigger className="flex items-center w-full px-2 py-1.5 hover:bg-accent rounded-md text-sm transition-colors">
          <div className="flex items-center flex-1 gap-2">
            {isOpen ? (
              <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
            ) : (
              <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
            )}
            <TableIcon className="h-3.5 w-3.5 text-blue-500" />
            <span className="font-medium truncate">{table.name}</span>
          </div>
          {table.row_count !== undefined && (
            <Badge variant="secondary" className="ml-2 text-xs">
              {table.row_count.toLocaleString()}
            </Badge>
          )}
        </CollapsibleTrigger>

        <CollapsibleContent className="ml-4 mt-1">
          <div className="space-y-0.5">
            {table.columns.map((column) => (
              <div
                key={column.name}
                className="flex items-center gap-2 px-2 py-1 text-xs hover:bg-accent/50 rounded-sm"
              >
                <div className="flex items-center gap-1.5 flex-1 min-w-0">
                  {column.is_primary_key && (
                    <Key className="h-3 w-3 text-yellow-500 flex-shrink-0" />
                  )}
                  {column.is_foreign_key && (
                    <Link2 className="h-3 w-3 text-purple-500 flex-shrink-0" />
                  )}
                  <span className="font-mono truncate">{column.name}</span>
                </div>
                <span className="text-muted-foreground text-xs whitespace-nowrap">
                  {column.data_type}
                </span>
              </div>
            ))}
          </div>
        </CollapsibleContent>
      </div>
    </Collapsible>
  );
}

export function SchemaBrowser() {
  const { schema, isLoading, error, loadSchema } = useSchemaStore();
  const { activeConnection } = useConnectionStore();
  const [exportDialogOpen, setExportDialogOpen] = useState(false);
  const [importDialogOpen, setImportDialogOpen] = useState(false);

  if (!activeConnection) {
    return (
      <div className="flex items-center justify-center h-full p-4 text-center">
        <p className="text-sm text-muted-foreground">
          Select a connection to view schema
        </p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full p-4 text-center">
        <p className="text-sm text-destructive">{error}</p>
      </div>
    );
  }

  if (isLoading && (!schema || schema.tables.length === 0)) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-4 text-center">
        <Loader2 className="h-8 w-8 animate-spin text-primary mb-2" />
        <p className="text-sm text-muted-foreground">Loading schema...</p>
      </div>
    );
  }

  if (!schema || schema.tables.length === 0) {
    return (
      <div className="flex items-center justify-center h-full p-4 text-center">
        <p className="text-sm text-muted-foreground">No tables found</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="p-4 border-b space-y-3">
        <div>
          <h3 className="font-semibold text-sm flex items-center gap-2">
            <TableIcon className="h-4 w-4 text-primary" />
            {schema.database_name || "Loading..."}
            {isLoading && (
              <Loader2 className="h-3 w-3 animate-spin text-muted-foreground ml-1" />
            )}
          </h3>
          <p className="text-xs text-muted-foreground mt-1">
            {schema.tables.length} {schema.tables.length === 1 ? "table" : "tables"}
            {isLoading && " (loading...)"}
          </p>
        </div>

        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            className="flex-1"
            onClick={() => setExportDialogOpen(true)}
          >
            <Download className="h-3.5 w-3.5 mr-1.5" />
            Export
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="flex-1"
            onClick={() => setImportDialogOpen(true)}
          >
            <Upload className="h-3.5 w-3.5 mr-1.5" />
            Import
          </Button>
        </div>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-2 space-y-1">
          {schema.tables.map((table) => (
            <TableItem
              key={table.name}
              table={table}
              onSelectTable={(table) => {
                // TODO: Handle table selection for query
                console.log("Selected table:", table.name);
              }}
            />
          ))}
        </div>
      </ScrollArea>

      <ExportDialog
        open={exportDialogOpen}
        onOpenChange={setExportDialogOpen}
        connectionId={activeConnection.id}
        tables={schema.tables}
      />

      <ImportDialog
        open={importDialogOpen}
        onOpenChange={setImportDialogOpen}
        connectionId={activeConnection.id}
        tables={schema.tables}
        onImportComplete={() => {
          // Reload schema after import
          loadSchema(activeConnection.id);
        }}
      />
    </div>
  );
}
