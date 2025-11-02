import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import * as dialog from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Progress } from "@/components/ui/progress";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { ErrorHandler } from "@/lib/ErrorHandler";
import { ImportOptions, ImportProgress } from "@/types/export.types";
import { Table } from "@/types/database.types";
import { AlertTriangle, Database, FileArchive, FileUp, StopCircle } from "lucide-react";

interface ImportDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  connectionId: string;
  databaseName: string;
  tables: Table[];
  onImportComplete?: () => void;
}

export function ImportDialog({
  open,
  onOpenChange,
  connectionId,
  databaseName,
  tables,
  onImportComplete,
}: ImportDialogProps) {
  const [sourcePath, setSourcePath] = useState<string>("");
  const [isZip, setIsZip] = useState<boolean>(false);
  const [tableMappings, setTableMappings] = useState<Record<string, string>>({});
  const [detectedFiles, setDetectedFiles] = useState<string[]>([]);
  const [isImporting, setIsImporting] = useState<boolean>(false);
  const [progress, setProgress] = useState<ImportProgress | null>(null);

  // Listen for progress events
  useEffect(() => {
    if (!isImporting) return;

    const unlisten = listen<ImportProgress>("import-progress", (event) => {
      setProgress(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [isImporting]);

  // Auto-detect mappings when files are detected
  useEffect(() => {
    if (detectedFiles.length > 0) {
      const newMappings: Record<string, string> = {};
      detectedFiles.forEach((fileName) => {
        // Try to match file name to table name
        const matchingTable = tables.find((t) => fileName === t.name);
        newMappings[fileName] = matchingTable?.name || fileName;
      });
      setTableMappings(newMappings);
    }
  }, [detectedFiles, tables]);

  const handleBrowse = async () => {
    try {
      const selected = await dialog.open({
        directory: false,
        multiple: false,
        title: "Select Import File",
        filters: [
          {
            name: "Import Files",
            extensions: ["zip", "csv"],
          },
        ],
      });

      if (selected && typeof selected === "string") {
        setSourcePath(selected);

        const isZipFile = selected.toLowerCase().endsWith(".zip");
        setIsZip(isZipFile);

        // If CSV, extract filename
        if (!isZipFile) {
          const fileName = selected.split("/").pop()?.replace(".csv", "") || "";
          setDetectedFiles([fileName]);
        } else {
          // For ZIP, we'll need to extract to see files
          // For now, we'll let the backend handle it
          setDetectedFiles([]);
        }
      }
    } catch (error) {
      ErrorHandler.handle(error, "Failed to select file");
    }
  };

  const handleImport = async () => {
    if (!sourcePath) {
      ErrorHandler.warning("Please select a file to import");
      return;
    }

    setIsImporting(true);
    setProgress(null);

    try {
      const options: ImportOptions = {
        connection_id: connectionId,
        source_path: sourcePath,
        is_zip: isZip,
        table_mappings: tableMappings,
      };

      await invoke("import_tables", { options });

      ErrorHandler.success("Import completed!", "All data has been imported successfully");
      onImportComplete?.();
      onOpenChange(false);
    } catch (error) {
      // Check if it was cancelled
      if (progress?.cancelled) {
        ErrorHandler.warning("Import cancelled");
      } else {
        ErrorHandler.handle(error, "Import failed");
      }
    } finally {
      setIsImporting(false);
      setProgress(null);
    }
  };

  const handleCancel = async () => {
    if (!isImporting) {
      onOpenChange(false);
      return;
    }

    try {
      await invoke("cancel_import", { connectionId });
      ErrorHandler.success("Cancelling import...");
    } catch (error) {
      ErrorHandler.handle(error, "Failed to cancel import");
    }
  };

  const progressPercent = progress
    ? Math.round((progress.current / progress.total) * 100)
    : 0;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Database className="h-5 w-5" />
            Import Data
          </DialogTitle>
          <DialogDescription>
            Import data from CSV files or ZIP archives into <span className="font-semibold text-foreground">{databaseName}</span>
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Source File */}
          <div className="space-y-2">
            <Label htmlFor="source-path">Source File</Label>
            <div className="flex gap-2">
              <Input
                id="source-path"
                value={sourcePath}
                onChange={(e) => setSourcePath(e.target.value)}
                placeholder="Select CSV or ZIP file..."
                disabled={isImporting}
                className="flex-1"
              />
              <Button
                variant="outline"
                onClick={handleBrowse}
                disabled={isImporting}
              >
                <FileUp className="h-4 w-4 mr-2" />
                Browse
              </Button>
            </div>
            {sourcePath && (
              <p className="text-sm text-muted-foreground flex items-center gap-2">
                {isZip ? (
                  <>
                    <FileArchive className="h-4 w-4" />
                    ZIP Archive
                  </>
                ) : (
                  <>
                    <FileUp className="h-4 w-4" />
                    CSV File
                  </>
                )}
              </p>
            )}
          </div>

          {/* Table Mappings */}
          {detectedFiles.length > 0 && (
            <div className="space-y-2">
              <Label>Table Mappings</Label>
              <ScrollArea className="h-48 border rounded-md p-4">
                <div className="space-y-3">
                  {detectedFiles.map((fileName) => (
                    <div key={fileName} className="flex items-center gap-2">
                      <span className="text-sm font-medium w-32 truncate">
                        {fileName}.csv
                      </span>
                      <span className="text-muted-foreground">→</span>
                      <Select
                        value={tableMappings[fileName] || fileName}
                        onValueChange={(value) =>
                          setTableMappings({ ...tableMappings, [fileName]: value })
                        }
                        disabled={isImporting}
                      >
                        <SelectTrigger className="flex-1">
                          <SelectValue placeholder="Select table" />
                        </SelectTrigger>
                        <SelectContent>
                          {tables.map((table) => (
                            <SelectItem key={table.name} value={table.name}>
                              {table.name}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            </div>
          )}

          {/* Progress */}
          {isImporting && progress && (
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">{progress.status}</span>
                <span className="font-medium">{progressPercent}%</span>
              </div>
              <div className="flex items-center gap-2">
                <Progress value={progressPercent} className="flex-1" />
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={handleCancel}
                  className="h-8 w-8 text-destructive hover:text-destructive hover:bg-destructive/10"
                  title="Cancel import"
                >
                  <StopCircle className="h-5 w-5" />
                </Button>
              </div>
              {progress.file_name && (
                <p className="text-sm text-muted-foreground">
                  Importing: {progress.file_name}
                </p>
              )}
            </div>
          )}

          {/* Warning */}
          {sourcePath && (
            <Alert variant="destructive">
              <AlertTriangle className="h-4 w-4" />
              <AlertTitle>Important Warning</AlertTitle>
              <AlertDescription className="space-y-1">
                <p>
                  This will insert data into the selected tables. Make sure the CSV columns match the table structure.
                </p>
                <p className="font-semibold">
                  Cancelling an import mid-operation may result in partial data and database corruption.
                  It is recommended to let the import complete or restore from a backup if cancelled.
                </p>
              </AlertDescription>
            </Alert>
          )}
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={handleCancel}
          >
            Cancel
          </Button>
          <Button onClick={handleImport} disabled={isImporting || !sourcePath}>
            {isImporting ? "Importing..." : "Import"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
