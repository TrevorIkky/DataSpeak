import { useEffect, useState } from "react";
import { Trash2, Link2, Upload, Download } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
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
import { useUIStore } from "@/stores/uiStore";
import { ExportDialog } from "@/components/export_import/ExportDialog";
import { ImportDialog } from "@/components/export_import/ImportDialog";
import type { Connection } from "@/types/database.types";

export function ConnectionList() {
  const {
    connections,
    activeConnection,
    setActiveConnection,
    deleteConnection,
    loadConnections,
  } = useConnectionStore();
  const { loadSchema, schema } = useSchemaStore();
  const {
    deleteConnectionDialogOpen,
    setDeleteConnectionDialogOpen,
    exportDialogOpen,
    setExportDialogOpen,
    importDialogOpen,
    setImportDialogOpen,
  } = useUIStore();
  const [connectionToDelete, setConnectionToDelete] = useState<Connection | null>(null);
  const [dialogConnection, setDialogConnection] = useState<Connection | null>(null);

  useEffect(() => {
    loadConnections();
  }, [loadConnections]);

  const handleConnect = async (connection: Connection) => {
    setActiveConnection(connection);
    await loadSchema(connection.id);
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

  const handleExportClick = async (connection: Connection) => {
    setDialogConnection(connection);
    // Load schema for the connection if not already loaded
    if (activeConnection?.id !== connection.id) {
      await loadSchema(connection.id);
    }
    setExportDialogOpen(true);
  };

  const handleImportClick = async (connection: Connection) => {
    setDialogConnection(connection);
    // Load schema for the connection if not already loaded
    if (activeConnection?.id !== connection.id) {
      await loadSchema(connection.id);
    }
    setImportDialogOpen(true);
  };

  if (connections.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-8 text-center">
        <h3 className="text-lg font-semibold mb-2">No Connections</h3>
        <p className="text-sm text-muted-foreground mb-4">
          Add a database connection to get started
        </p>
      </div>
    );
  }

  return (
    <>
      <ScrollArea className="h-full">
        <div className="p-4">
          <Accordion type="single" collapsible className="space-y-2">
            {connections.map((connection) => {
              const isActive = activeConnection?.id === connection.id;

              return (
                <ContextMenu key={connection.id}>
                  <ContextMenuTrigger>
                    <AccordionItem
                      value={connection.id}
                      className={`border rounded-lg transition-all ${
                        isActive ? "border-primary bg-primary/5" : ""
                      }`}
                    >
                      <AccordionTrigger className="px-4 py-3 hover:no-underline">
                        <div className="flex items-center justify-between w-full pr-2">
                          <div className="flex items-center gap-3">
                            <span className="font-semibold text-sm">{connection.name}</span>
                          </div>
                          <div className="flex items-center gap-2 text-xs text-muted-foreground">
                            <span>{connection.host}:{connection.port}</span>
                          </div>
                        </div>
                      </AccordionTrigger>
                      <AccordionContent className="px-4 pb-4">
                        <div className="space-y-3 pt-2">
                          <div className="grid grid-cols-2 gap-2 text-sm">
                            <div>
                              <p className="text-xs text-muted-foreground">Database Type</p>
                              <p className="font-medium">{connection.database_type}</p>
                            </div>
                            <div>
                              <p className="text-xs text-muted-foreground">Default Database</p>
                              <p className="font-medium truncate">{connection.default_database}</p>
                            </div>
                            <div>
                              <p className="text-xs text-muted-foreground">Host</p>
                              <p className="font-medium">{connection.host}</p>
                            </div>
                            <div>
                              <p className="text-xs text-muted-foreground">Port</p>
                              <p className="font-medium">{connection.port}</p>
                            </div>
                            <div>
                              <p className="text-xs text-muted-foreground">Username</p>
                              <p className="font-medium">{connection.username}</p>
                            </div>
                          </div>

                          {!isActive && (
                            <div className="pt-2 border-t">
                              <Button
                                size="sm"
                                className="w-full"
                                onClick={() => handleConnect(connection)}
                              >
                                <Link2 className="h-3.5 w-3.5 mr-1.5" />
                                Connect
                              </Button>
                            </div>
                          )}
                        </div>
                      </AccordionContent>
                    </AccordionItem>
                  </ContextMenuTrigger>
                  <ContextMenuContent>
                    {!isActive && (
                      <>
                        <ContextMenuItem onClick={() => handleConnect(connection)}>
                          <Link2 className="h-4 w-4 mr-2" />
                          Connect
                        </ContextMenuItem>
                        <ContextMenuSeparator />
                      </>
                    )}
                    <ContextMenuItem onClick={() => handleExportClick(connection)}>
                      <Download className="h-4 w-4 mr-2" />
                      Export Tables
                    </ContextMenuItem>
                    <ContextMenuItem onClick={() => handleImportClick(connection)}>
                      <Upload className="h-4 w-4 mr-2" />
                      Import Tables
                    </ContextMenuItem>
                    <ContextMenuSeparator />
                    <ContextMenuItem
                      onClick={() => handleDeleteClick(connection)}
                      className="text-destructive focus:text-destructive focus:bg-destructive/10"
                    >
                      <Trash2 className="h-4 w-4 mr-2" />
                      Delete Connection
                    </ContextMenuItem>
                  </ContextMenuContent>
                </ContextMenu>
              );
            })}
          </Accordion>
        </div>
      </ScrollArea>

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

      {dialogConnection && schema && (
        <>
          <ExportDialog
            open={exportDialogOpen}
            onOpenChange={setExportDialogOpen}
            connectionId={dialogConnection.id}
            tables={schema.tables}
          />

          <ImportDialog
            open={importDialogOpen}
            onOpenChange={setImportDialogOpen}
            connectionId={dialogConnection.id}
            tables={schema.tables}
            onImportComplete={async () => {
              // Reload schema after import
              await loadSchema(dialogConnection.id);
            }}
          />
        </>
      )}
    </>
  );
}
