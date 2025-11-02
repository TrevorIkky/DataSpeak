import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import * as dialog from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ErrorHandler } from "@/lib/ErrorHandler";
import { ExportOptions, ExportProgress } from "@/types/export.types";
import { Table } from "@/types/database.types";
import { Database, FileArchive, FolderOpen, StopCircle } from "lucide-react";

interface ExportDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  connectionId: string;
  tables: Table[];
  databaseName?: string;
}

export function ExportDialog({ open, onOpenChange, connectionId, tables, databaseName }: ExportDialogProps) {
  const [selectedTables, setSelectedTables] = useState<Set<string>>(new Set());
  const [outputDir, setOutputDir] = useState<string>("");
  const [createZip, setCreateZip] = useState<boolean>(true);
  const [isExporting, setIsExporting] = useState<boolean>(false);
  const [progress, setProgress] = useState<ExportProgress | null>(null);

  // Select all tables by default
  useEffect(() => {
    if (tables.length > 0) {
      setSelectedTables(new Set(tables.map((t) => t.name)));
    }
  }, [tables]);

  // Clear output path when ZIP option changes
  useEffect(() => {
    setOutputDir("");
  }, [createZip]);

  // Listen for progress events
  useEffect(() => {
    if (!isExporting) return;

    const unlisten = listen<ExportProgress>("export-progress", (event) => {
      setProgress(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [isExporting]);

  const handleSelectAll = () => {
    setSelectedTables(new Set(tables.map((t) => t.name)));
  };

  const handleDeselectAll = () => {
    setSelectedTables(new Set());
  };

  const handleTableToggle = (tableName: string) => {
    const newSelected = new Set(selectedTables);
    if (newSelected.has(tableName)) {
      newSelected.delete(tableName);
    } else {
      newSelected.add(tableName);
    }
    setSelectedTables(newSelected);
  };

  const handleBrowse = async () => {
    try {
      if (createZip) {
        // Generate default filename with database name and epoch timestamp
        const timestamp = Math.floor(Date.now() / 1000);
        const dbName = databaseName || 'export';
        const defaultFilename = `${dbName}_${timestamp}.zip`;

        // For ZIP: let user select file location
        const selected = await dialog.save({
          title: "Save ZIP Archive",
          defaultPath: defaultFilename,
          filters: [
            {
              name: "ZIP Archive",
              extensions: ["zip"],
            },
          ],
        });

        if (selected && typeof selected === "string") {
          setOutputDir(selected);
        }
      } else {
        // For CSV: let user select directory
        const selected = await dialog.open({
          directory: true,
          multiple: false,
          title: "Select Output Directory",
        });

        if (selected && typeof selected === "string") {
          setOutputDir(selected);
        }
      }
    } catch (error) {
      ErrorHandler.handle(error, "Failed to select location");
    }
  };

  const handleExport = async () => {
    if (selectedTables.size === 0) {
      ErrorHandler.warning("Please select at least one table to export");
      return;
    }

    if (!outputDir) {
      ErrorHandler.warning("Please select an output directory");
      return;
    }

    setIsExporting(true);
    setProgress(null);

    try {
      const options: ExportOptions = {
        connection_id: connectionId,
        tables: Array.from(selectedTables),
        output_dir: outputDir,
        create_zip: createZip,
      };

      const resultPath = await invoke<string>("export_tables", { options });

      ErrorHandler.success("Export completed!", `Saved to: ${resultPath}`);
      onOpenChange(false);
    } catch (error) {
      // Check if it was cancelled
      if (progress?.cancelled) {
        ErrorHandler.warning("Export cancelled");
      } else {
        ErrorHandler.handle(error, "Export failed");
      }
    } finally {
      setIsExporting(false);
      setProgress(null);
    }
  };

  const handleCancel = async () => {
    if (!isExporting) {
      onOpenChange(false);
      return;
    }

    try {
      await invoke("cancel_export", { connectionId });
      ErrorHandler.success("Cancelling export...");
    } catch (error) {
      ErrorHandler.handle(error, "Failed to cancel export");
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
            Export Tables
          </DialogTitle>
          <DialogDescription>
            Export selected tables to CSV files with optional ZIP compression
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Table Selection */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label>Select Tables ({selectedTables.size} of {tables.length})</Label>
              <div className="flex gap-2">
                <Button variant="ghost" size="sm" onClick={handleSelectAll}>
                  Select All
                </Button>
                <Button variant="ghost" size="sm" onClick={handleDeselectAll}>
                  Deselect All
                </Button>
              </div>
            </div>

            <ScrollArea className="h-48 border rounded-md p-4">
              <div className="space-y-2">
                {tables.map((table) => (
                  <div key={table.name} className="flex items-center space-x-2">
                    <Checkbox
                      id={`table-${table.name}`}
                      checked={selectedTables.has(table.name)}
                      onCheckedChange={() => handleTableToggle(table.name)}
                      disabled={isExporting}
                    />
                    <label
                      htmlFor={`table-${table.name}`}
                      className="flex-1 text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 cursor-pointer"
                    >
                      {table.name}
                      {table.row_count !== null && table.row_count !== undefined && (
                        <span className="ml-2 text-muted-foreground">
                          ({table.row_count.toLocaleString()} rows)
                        </span>
                      )}
                    </label>
                  </div>
                ))}
              </div>
            </ScrollArea>
          </div>

          {/* Output Location */}
          <div className="space-y-2">
            <Label htmlFor="output-dir">
              {createZip ? "Output File" : "Output Directory"}
            </Label>
            <div className="flex gap-2">
              <Input
                id="output-dir"
                value={outputDir}
                onChange={(e) => setOutputDir(e.target.value)}
                placeholder={createZip ? "Select ZIP file location..." : "Select output directory..."}
                disabled={isExporting}
                className="flex-1"
              />
              <Button
                variant="outline"
                onClick={handleBrowse}
                disabled={isExporting}
              >
                <FolderOpen className="h-4 w-4 mr-2" />
                Browse
              </Button>
            </div>
          </div>

          {/* ZIP Option */}
          <div className="flex items-center space-x-2">
            <Checkbox
              id="create-zip"
              checked={createZip}
              onCheckedChange={(checked) => setCreateZip(checked as boolean)}
              disabled={isExporting}
            />
            <label
              htmlFor="create-zip"
              className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 cursor-pointer flex items-center gap-2"
            >
              <FileArchive className="h-4 w-4" />
              Create ZIP archive
            </label>
          </div>

          {/* Progress */}
          {isExporting && progress && (
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
                  title="Cancel export"
                >
                  <StopCircle className="h-5 w-5" />
                </Button>
              </div>
              {progress.table_name && (
                <p className="text-sm text-muted-foreground">
                  Exporting: {progress.table_name}
                </p>
              )}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={handleCancel}
          >
            Cancel
          </Button>
          <Button onClick={handleExport} disabled={isExporting}>
            {isExporting ? "Exporting..." : "Export"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
