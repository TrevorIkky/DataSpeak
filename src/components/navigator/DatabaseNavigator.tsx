import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
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
  Columns3,
  Key,
  Shield,
  Zap,
  ListTree,
  DatabaseZap,
  Eraser,
  RefreshCw
} from "lucide-react";
import { ErrorHandler } from "@/lib/ErrorHandler";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
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
import type { Connection, Table, Column } from "@/types/database.types";

interface TableItemProps {
  table: Table;
  onTableClick: (table: Table) => void;
}

function TableItem({ table, onTableClick }: TableItemProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [columnsOpen, setColumnsOpen] = useState(true);
  const [foreignKeysOpen, setForeignKeysOpen] = useState(false);
  const [constraintsOpen, setConstraintsOpen] = useState(false);
  const [triggersOpen, setTriggersOpen] = useState(false);
  const [indexesOpen, setIndexesOpen] = useState(false);

  const foreignKeys = table.columns.filter(col => col.is_foreign_key);

  return (
    <ContextMenu>
      <ContextMenuTrigger>
        <Collapsible open={isOpen} onOpenChange={setIsOpen}>
          <div className="mb-1">
            <div className="flex items-center group">
              <CollapsibleTrigger
                className="flex items-center justify-center p-1 hover:bg-accent rounded flex-shrink-0"
                onClick={(e) => e.stopPropagation()}
              >
                {isOpen ? (
                  <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
                ) : (
                  <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
                )}
              </CollapsibleTrigger>
              <button
                onClick={() => onTableClick(table)}
                className="flex items-center gap-2 px-2 py-1.5 hover:bg-accent rounded flex-1 min-w-0"
              >
                <TableIcon className="h-3.5 w-3.5 text-blue-500 flex-shrink-0" />
                <span className="font-medium truncate text-sm text-left">
                  {table.name}
                </span>
                {table.row_count !== undefined && (
                  <Badge variant="secondary" className="text-xs h-5 ml-auto flex-shrink-0">
                    {table.row_count.toLocaleString()}
                  </Badge>
                )}
              </button>
            </div>

            <CollapsibleContent>
              <div className="ml-5 pl-3 border-l-2 border-border/40 text-xs">
                {/* Columns Section */}
                <Collapsible open={columnsOpen} onOpenChange={setColumnsOpen}>
                  <div className="flex items-center py-1.5">
                    <CollapsibleTrigger
                      className="flex items-center justify-center p-0.5 hover:bg-accent rounded flex-shrink-0"
                      onClick={(e) => e.stopPropagation()}
                    >
                      {columnsOpen ? (
                        <ChevronDown className="h-3 w-3 text-muted-foreground" />
                      ) : (
                        <ChevronRight className="h-3 w-3 text-muted-foreground" />
                      )}
                    </CollapsibleTrigger>
                    <div className="flex items-center gap-1.5 px-2 text-muted-foreground">
                      <Columns3 className="h-3 w-3" />
                      <span className="font-medium">Columns ({table.columns.length})</span>
                    </div>
                  </div>
                  <CollapsibleContent>
                    <div className="space-y-1 ml-5 pb-1">
                      {table.columns.map((col: Column) => (
                        <div key={col.name} className="flex items-center gap-2 text-xs py-0.5">
                          <span className="font-mono text-foreground">{col.name}</span>
                          <span className="text-muted-foreground">{col.data_type}</span>
                          {col.is_primary_key && (
                            <Badge variant="outline" className="text-xs h-4 px-1">PK</Badge>
                          )}
                          {col.is_foreign_key && (
                            <Badge variant="outline" className="text-xs h-4 px-1">FK</Badge>
                          )}
                          {!col.is_nullable && (
                            <span className="text-muted-foreground">NOT NULL</span>
                          )}
                        </div>
                      ))}
                    </div>
                  </CollapsibleContent>
                </Collapsible>

                {/* Foreign Keys Section */}
                <Collapsible open={foreignKeysOpen} onOpenChange={setForeignKeysOpen}>
                  <div className="flex items-center py-1.5">
                    <CollapsibleTrigger
                      className="flex items-center justify-center p-0.5 hover:bg-accent rounded flex-shrink-0"
                      onClick={(e) => e.stopPropagation()}
                    >
                      {foreignKeysOpen ? (
                        <ChevronDown className="h-3 w-3 text-muted-foreground" />
                      ) : (
                        <ChevronRight className="h-3 w-3 text-muted-foreground" />
                      )}
                    </CollapsibleTrigger>
                    <div className="flex items-center gap-1.5 px-2 text-muted-foreground">
                      <Key className="h-3 w-3" />
                      <span className="font-medium">Foreign Keys ({foreignKeys.length})</span>
                    </div>
                  </div>
                  <CollapsibleContent>
                    {foreignKeys.length > 0 ? (
                      <div className="space-y-1 ml-5 pb-1">
                        {foreignKeys.map((col: Column) => (
                          <div key={col.name} className="text-xs py-0.5">
                            <span className="font-mono text-foreground">{col.name}</span>
                            <span className="text-muted-foreground"> → </span>
                            <span className="text-primary">
                              {col.foreign_key_table}.{col.foreign_key_column}
                            </span>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <div className="ml-5 pb-1 text-muted-foreground">No foreign keys</div>
                    )}
                  </CollapsibleContent>
                </Collapsible>

                {/* Constraints Section */}
                <Collapsible open={constraintsOpen} onOpenChange={setConstraintsOpen}>
                  <div className="flex items-center py-1.5">
                    <CollapsibleTrigger
                      className="flex items-center justify-center p-0.5 hover:bg-accent rounded flex-shrink-0"
                      onClick={(e) => e.stopPropagation()}
                    >
                      {constraintsOpen ? (
                        <ChevronDown className="h-3 w-3 text-muted-foreground" />
                      ) : (
                        <ChevronRight className="h-3 w-3 text-muted-foreground" />
                      )}
                    </CollapsibleTrigger>
                    <div className="flex items-center gap-1.5 px-2 text-muted-foreground">
                      <Shield className="h-3 w-3" />
                      <span className="font-medium">Constraints ({table.constraints?.length || 0})</span>
                    </div>
                  </div>
                  <CollapsibleContent>
                    {table.constraints && table.constraints.length > 0 ? (
                      <div className="space-y-1 ml-5 pb-1">
                        {table.constraints.map((constraint) => (
                          <div key={constraint.name} className="text-xs">
                            <div className="font-medium text-foreground">{constraint.name}</div>
                            <div className="text-muted-foreground ml-2">
                              {constraint.constraint_type} ({constraint.columns.join(', ')})
                              {constraint.referenced_table && constraint.referenced_columns && (
                                <span> → {constraint.referenced_table}({constraint.referenced_columns.join(', ')})</span>
                              )}
                            </div>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <div className="ml-5 pb-1 text-muted-foreground text-xs">No constraints</div>
                    )}
                  </CollapsibleContent>
                </Collapsible>

                {/* Triggers Section */}
                <Collapsible open={triggersOpen} onOpenChange={setTriggersOpen}>
                  <div className="flex items-center py-1.5">
                    <CollapsibleTrigger
                      className="flex items-center justify-center p-0.5 hover:bg-accent rounded flex-shrink-0"
                      onClick={(e) => e.stopPropagation()}
                    >
                      {triggersOpen ? (
                        <ChevronDown className="h-3 w-3 text-muted-foreground" />
                      ) : (
                        <ChevronRight className="h-3 w-3 text-muted-foreground" />
                      )}
                    </CollapsibleTrigger>
                    <div className="flex items-center gap-1.5 px-2 text-muted-foreground">
                      <Zap className="h-3 w-3" />
                      <span className="font-medium">Triggers ({table.triggers?.length || 0})</span>
                    </div>
                  </div>
                  <CollapsibleContent>
                    {table.triggers && table.triggers.length > 0 ? (
                      <div className="space-y-1 ml-5 pb-1">
                        {table.triggers.map((trigger) => (
                          <div key={trigger.name} className="text-xs">
                            <div className="font-medium text-foreground">{trigger.name}</div>
                            <div className="text-muted-foreground ml-2">
                              {trigger.timing} {trigger.event}
                            </div>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <div className="ml-5 pb-1 text-muted-foreground text-xs">No triggers</div>
                    )}
                  </CollapsibleContent>
                </Collapsible>

                {/* Indexes Section */}
                <Collapsible open={indexesOpen} onOpenChange={setIndexesOpen}>
                  <div className="flex items-center py-1.5">
                    <CollapsibleTrigger
                      className="flex items-center justify-center p-0.5 hover:bg-accent rounded flex-shrink-0"
                      onClick={(e) => e.stopPropagation()}
                    >
                      {indexesOpen ? (
                        <ChevronDown className="h-3 w-3 text-muted-foreground" />
                      ) : (
                        <ChevronRight className="h-3 w-3 text-muted-foreground" />
                      )}
                    </CollapsibleTrigger>
                    <div className="flex items-center gap-1.5 px-2 text-muted-foreground">
                      <ListTree className="h-3 w-3" />
                      <span className="font-medium">Indexes ({table.indexes?.length || 0})</span>
                    </div>
                  </div>
                  <CollapsibleContent>
                    {table.indexes && table.indexes.length > 0 ? (
                      <div className="space-y-1 ml-5 pb-1">
                        {table.indexes.map((index) => (
                          <div key={index.name} className="text-xs">
                            <div className="flex items-center gap-1.5">
                              <span className="font-medium text-foreground">{index.name}</span>
                              {index.is_primary && (
                                <Badge variant="outline" className="text-xs h-4 px-1">PRIMARY</Badge>
                              )}
                              {index.is_unique && !index.is_primary && (
                                <Badge variant="outline" className="text-xs h-4 px-1">UNIQUE</Badge>
                              )}
                            </div>
                            <div className="text-muted-foreground ml-2">
                              {index.index_type && `${index.index_type} `}({index.columns.join(', ')})
                            </div>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <div className="ml-5 pb-1 text-muted-foreground text-xs">No indexes</div>
                    )}
                  </CollapsibleContent>
                </Collapsible>
              </div>
            </CollapsibleContent>
          </div>
        </Collapsible>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onClick={() => onTableClick(table)}>
          <TableIcon className="h-4 w-4 mr-2" />
          View Data
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}

interface ConnectionItemProps {
  connection: Connection;
  isActive: boolean;
  isOpen?: boolean;
  onToggle?: (isOpen: boolean) => void;
  onConnect: (connection: Connection) => void;
  onEdit: (connection: Connection) => void;
  onDelete: (connection: Connection) => void;
}

function ConnectionItem({ connection, isActive, isOpen: controlledIsOpen, onToggle, onConnect, onEdit, onDelete }: ConnectionItemProps) {
  const [clearDataDialogOpen, setClearDataDialogOpen] = useState(false);
  const [clearDatabaseDialogOpen, setClearDatabaseDialogOpen] = useState(false);
  const { schema, isLoading: schemaLoading } = useSchemaStore();
  const { addTableTab } = useQueryStore();
  const { activeConnection } = useConnectionStore();
  const {
    exportDialogOpen,
    setExportDialogOpen,
    importDialogOpen,
    setImportDialogOpen,
  } = useUIStore();

  const isOpen = controlledIsOpen ?? false;

  const handleOpenChange = (newOpen: boolean) => {
    if (onToggle) {
      onToggle(newOpen);
    }
  };

  const handleTableClick = (table: Table) => {
    addTableTab(table.name);
  };

  const handleClearData = async () => {
    if (!activeConnection) return;

    try {
      const { loadSchema } = useSchemaStore.getState();

      await invoke("clear_data_only", {
        connectionId: activeConnection.id,
      });

      await loadSchema(activeConnection.id);
      ErrorHandler.success("Data cleared", "All data has been cleared from all tables");
    } catch (error) {
      ErrorHandler.handle(error, "Failed to clear data");
    } finally {
      setClearDataDialogOpen(false);
    }
  };

  const handleClearDatabase = async () => {
    if (!activeConnection) return;

    try {
      const { loadSchema } = useSchemaStore.getState();

      await invoke("clear_database", {
        connectionId: activeConnection.id,
      });

      await loadSchema(activeConnection.id);
      ErrorHandler.success("Database cleared", "All tables have been removed from the database");
    } catch (error) {
      ErrorHandler.handle(error, "Failed to clear database");
    } finally {
      setClearDatabaseDialogOpen(false);
    }
  };

  const handleRefresh = async () => {
    if (!activeConnection) return;

    try {
      const { loadSchema } = useSchemaStore.getState();
      await loadSchema(activeConnection.id);
      ErrorHandler.success("Schema refreshed", "Database schema has been reloaded");
    } catch (error) {
      ErrorHandler.handle(error, "Failed to refresh schema");
    }
  };

  const showSchema = isActive && activeConnection?.id === connection.id;

  return (
    <>
      <ContextMenu>
        <ContextMenuTrigger>
          <Collapsible open={isOpen} onOpenChange={handleOpenChange}>
            <div className="rounded-lg mb-2 transition-all">
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
                {showSchema ? (
                  <div className="px-2 pb-2">
                    {/* Tables List */}
                    {schemaLoading && (!schema || schema.tables.length === 0) ? (
                      <div className="space-y-2 px-2 py-2">
                        {Array.from({ length: 6 }).map((_, i) => (
                          <Skeleton key={i} className="h-6 w-full mx-4" />
                        ))}
                      </div>
                    ) : schema && schema.tables.length > 0 ? (
                      <div className="space-y-1">
                        <div className="px-2 py-1 text-xs text-muted-foreground">
                          {schema.tables.length} {schema.tables.length === 1 ? "table" : "tables"}
                        </div>
                        {schema.tables.map((table) => (
                          <TableItem
                            key={table.name}
                            table={table}
                            onTableClick={handleTableClick}
                          />
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
          {isActive && (
            <ContextMenuItem onClick={handleRefresh}>
              <RefreshCw className="h-4 w-4 mr-2" />
              Refresh Schema
            </ContextMenuItem>
          )}
          <ContextMenuSeparator />
          <ContextMenuItem
            onClick={() => setClearDataDialogOpen(true)}
            className="text-destructive focus:text-destructive focus:bg-destructive/10"
          >
            <Eraser className="h-4 w-4 mr-2" />
            Clear All Data
          </ContextMenuItem>
          <ContextMenuItem
            onClick={() => setClearDatabaseDialogOpen(true)}
            className="text-destructive focus:text-destructive focus:bg-destructive/10"
          >
            <DatabaseZap className="h-4 w-4 mr-2" />
            Drop All Tables
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
            databaseName={activeConnection.default_database || activeConnection.name}
          />

          <ImportDialog
            open={importDialogOpen}
            onOpenChange={setImportDialogOpen}
            connectionId={activeConnection.id}
            databaseName={activeConnection.default_database || activeConnection.name}
            tables={schema.tables}
            onImportComplete={() => {
              // Schema will be reloaded automatically
            }}
          />
        </>
      )}

      {/* Clear Data Confirmation Dialog */}
      <AlertDialog open={clearDataDialogOpen} onOpenChange={setClearDataDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Clear All Data</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to clear all data from "{connection.default_database || connection.name}"?
              This will delete all data from all tables but keep the table structures intact.
              This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleClearData}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Clear All Data
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Drop All Tables Confirmation Dialog */}
      <AlertDialog open={clearDatabaseDialogOpen} onOpenChange={setClearDatabaseDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Drop All Tables</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to drop all tables from "{connection.default_database || connection.name}"?
              This will permanently remove all tables and their data from the database.
              This action cannot be undone and will result in a completely empty database.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleClearDatabase}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Drop All Tables
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
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
  const {
    setConnectionDialogOpen,
    deleteConnectionDialogOpen,
    setDeleteConnectionDialogOpen,
    openConnectionId,
    setOpenConnectionId,
  } = useUIStore();
  const [connectionToDelete, setConnectionToDelete] = useState<Connection | null>(null);

  useEffect(() => {
    loadConnections();
  }, [loadConnections]);

  const handleConnect = async (connection: Connection) => {
    setActiveConnection(connection);
    await loadSchema(connection.id);
  };

  const handleConnectionToggle = async (connection: Connection, isOpen: boolean) => {
    if (isOpen) {
      // Close all other connections and open this one
      setOpenConnectionId(connection.id);
      // Auto-connect when opening
      if (activeConnection?.id !== connection.id) {
        setActiveConnection(connection);
        await loadSchema(connection.id);
      }
    } else {
      setOpenConnectionId(null);
    }
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
          <div className="p-3 safe-area-bottom">
            {connections.map((connection) => (
              <ConnectionItem
                key={connection.id}
                connection={connection}
                isActive={activeConnection?.id === connection.id}
                isOpen={openConnectionId === connection.id}
                onToggle={(isOpen) => handleConnectionToggle(connection, isOpen)}
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
