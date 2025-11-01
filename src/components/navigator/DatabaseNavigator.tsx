import { useEffect, useState } from "react";
import {
  ChevronRight,
  ChevronDown,
  Database,
  Table as TableIcon,
  Trash2,
  Link2,
  Edit,
  Download,
  Upload,
  Loader2
} from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { useConnectionStore } from "@/stores/connectionStore";
import { useSchemaStore } from "@/stores/schemaStore";
import { useQueryStore } from "@/stores/queryStore";
import { useUIStore } from "@/stores/uiStore";
import { ExportDialog } from "@/components/export_import/ExportDialog";
import { ImportDialog } from "@/components/export_import/ImportDialog";
import type { Connection, Table } from "@/types/database.types";

interface ConnectionItemProps {
  connection: Connection;
  isActive: boolean;
  onConnect: (connection: Connection) => void;
  onEdit: (connection: Connection) => void;
  onDelete: (connection: Connection) => void;
}

function ConnectionItem({ connection, isActive, onConnect, onEdit, onDelete }: ConnectionItemProps) {
  const [isOpen, setIsOpen] = useState(false);
  const { schema, isLoading: schemaLoading } = useSchemaStore();
  const { addTableTab } = useQueryStore();
  const { activeConnection } = useConnectionStore();
  const {
    exportDialogOpen,
    setExportDialogOpen,
    importDialogOpen,
    setImportDialogOpen,
  } = useUIStore();

  // Auto-expand when connection becomes active
  useEffect(() => {
    if (isActive) {
      setIsOpen(true);
    }
  }, [isActive]);

  const handleTableClick = (table: Table) => {
    addTableTab(table.name);
  };

  const showSchema = isActive && activeConnection?.id === connection.id;

  return (
    <>
      <ContextMenu>
        <ContextMenuTrigger>
          <Collapsible open={isOpen} onOpenChange={setIsOpen}>
            <div
              className={`rounded-lg mb-2 transition-all ${isActive ? "bg-primary/5" : ""
                }`}
            >
              <CollapsibleTrigger className="w-full px-3 py-2.5 flex items-center justify-between hover:bg-accent/50 rounded-lg transition-colors">
                <div className="flex items-center gap-2 flex-1 min-w-0">
                  {isOpen ? (
                    <ChevronDown className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                  ) : (
                    <ChevronRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                  )}
                  <Database className={`h-4 w-4 flex-shrink-0 ${isActive ? "text-primary" : "text-muted-foreground"}`} />
                  <span className="font-semibold text-sm truncate">{connection.name}</span>
                  {isActive && (
                    <div className="h-2 w-2 rounded-full bg-green-500 animate-pulse flex-shrink-0" />
                  )}
                </div>
              </CollapsibleTrigger>

              <CollapsibleContent className="overflow-hidden">
                {!isActive ? (
                  <div className="px-3 pb-3 pt-1">
                    <Button
                      size="sm"
                      className="w-full"
                      onClick={() => onConnect(connection)}
                    >
                      <Link2 className="h-3.5 w-3.5 mr-1.5" />
                      Connect
                    </Button>
                  </div>
                ) : showSchema ? (
                  <div className="px-2 pb-2">
                    {/* Tables List */}
                    {schemaLoading && (!schema || schema.tables.length === 0) ? (
                      <div className="flex flex-col items-center justify-center py-4 text-center">
                        <Loader2 className="h-6 w-6 animate-spin text-primary mb-2" />
                        <p className="text-xs text-muted-foreground">Loading schema...</p>
                      </div>
                    ) : schema && schema.tables.length > 0 ? (
                      <div className="space-y-0.5">
                        <div className="px-2 py-1 text-xs text-muted-foreground">
                          {schema.tables.length} {schema.tables.length === 1 ? "table" : "tables"}
                        </div>
                        {schema.tables.map((table) => (
                          <ContextMenu key={table.name}>
                            <ContextMenuTrigger>
                              <button
                                onClick={() => handleTableClick(table)}
                                className="w-full flex items-center gap-2 px-2 py-1.5 hover:bg-accent rounded-md text-sm transition-colors group"
                              >
                                <TableIcon className="h-3.5 w-3.5 text-blue-500 flex-shrink-0" />
                                <span className="font-medium truncate flex-1 text-left">
                                  {table.name}
                                </span>
                                {table.row_count !== undefined && (
                                  <Badge variant="secondary" className="text-xs h-5">
                                    {table.row_count.toLocaleString()}
                                  </Badge>
                                )}
                              </button>
                            </ContextMenuTrigger>
                            <ContextMenuContent>
                              <ContextMenuItem onClick={() => handleTableClick(table)}>
                                <TableIcon className="h-4 w-4 mr-2" />
                                View Data
                              </ContextMenuItem>
                            </ContextMenuContent>
                          </ContextMenu>
                        ))}
                      </div>
                    ) : (
                      <div className="px-2 py-4 text-center">
                        <p className="text-xs text-muted-foreground">No tables found</p>
                      </div>
                    )}
                  </div>
                ) : null}
              </CollapsibleContent>
            </div>
          </Collapsible>
        </ContextMenuTrigger>
        <ContextMenuContent>
          {!isActive ? (
            <>
              <ContextMenuItem onClick={() => onConnect(connection)}>
                <Link2 className="h-4 w-4 mr-2" />
                Connect
              </ContextMenuItem>
              <ContextMenuSeparator />
            </>
          ) : null}
          <ContextMenuItem onClick={() => onEdit(connection)}>
            <Edit className="h-4 w-4 mr-2" />
            Edit Connection
          </ContextMenuItem>
          <ContextMenuSeparator />
          <ContextMenuItem onClick={() => setExportDialogOpen(true)}>
            <Download className="h-4 w-4 mr-2" />
            Export Tables
          </ContextMenuItem>
          <ContextMenuItem onClick={() => setImportDialogOpen(true)}>
            <Upload className="h-4 w-4 mr-2" />
            Import Tables
          </ContextMenuItem>
          <ContextMenuSeparator />
          <ContextMenuItem
            onClick={() => onDelete(connection)}
            className="text-destructive focus:text-destructive focus:bg-destructive/10"
          >
            <Trash2 className="h-4 w-4 mr-2" />
            Delete Connection
          </ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>

      {/* Export/Import Dialogs */}
      {schema && activeConnection && (
        <>
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
              // Schema will be reloaded automatically
            }}
          />
        </>
      )}
    </>
  );
}

export function DatabaseNavigator() {
  const {
    connections,
    activeConnection,
    setActiveConnection,
    deleteConnection,
    loadConnections,
  } = useConnectionStore();
  const { loadSchema } = useSchemaStore();
  const { setConnectionDialogOpen, deleteConnectionDialogOpen, setDeleteConnectionDialogOpen } = useUIStore();
  const [connectionToDelete, setConnectionToDelete] = useState<Connection | null>(null);

  useEffect(() => {
    loadConnections();
  }, [loadConnections]);

  const handleConnect = async (connection: Connection) => {
    setActiveConnection(connection);
    await loadSchema(connection.id);
  };

  const handleEdit = (connection: Connection) => {
    setConnectionDialogOpen(true, connection.id);
  };

  const handleDeleteClick = (connection: Connection) => {
    setConnectionToDelete(connection);
    setDeleteConnectionDialogOpen(true);
  };

  const handleDeleteConfirm = async () => {
    if (connectionToDelete) {
      await deleteConnection(connectionToDelete.id);
      setDeleteConnectionDialogOpen(false);
      setConnectionToDelete(null);
    }
  };

  if (connections.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-8 text-center">
        <Database className="h-12 w-12 text-muted-foreground mb-3" />
        <h3 className="text-sm font-semibold mb-1">No Connections</h3>
        <p className="text-xs text-muted-foreground mb-4">
          Add a database connection to get started
        </p>
      </div>
    );
  }

  return (
    <>
      <div className="flex flex-col h-full">
        <div className="px-4 py-3 border-b">
          <h2 className="font-semibold text-sm uppercase tracking-wide text-muted-foreground">
            Database Navigator
          </h2>
        </div>

        <ScrollArea className="flex-1">
          <div className="p-3">
            {connections.map((connection) => (
              <ConnectionItem
                key={connection.id}
                connection={connection}
                isActive={activeConnection?.id === connection.id}
                onConnect={handleConnect}
                onEdit={handleEdit}
                onDelete={handleDeleteClick}
              />
            ))}
          </div>
        </ScrollArea>
      </div>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={deleteConnectionDialogOpen} onOpenChange={setDeleteConnectionDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Connection</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete "{connectionToDelete?.name}"? This
              action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDeleteConfirm}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
