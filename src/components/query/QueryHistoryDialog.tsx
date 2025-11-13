import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Clock, Check, X, Trash2 } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { useConnectionStore } from "@/stores/connectionStore";
import { useSchemaStore } from "@/stores/schemaStore";
import { ErrorHandler } from "@/lib/ErrorHandler";
import { highlightSQL } from "@/lib/sqlSyntaxHighlight";
import type { QueryHistoryEntry } from "@/types/query-history.types";

interface QueryHistoryDialogProps {
  onSelectQuery: (query: string) => void;
}

export function QueryHistoryDialog({ onSelectQuery }: QueryHistoryDialogProps) {
  const { activeConnection } = useConnectionStore();
  const { schema, keywords } = useSchemaStore();
  const [history, setHistory] = useState<QueryHistoryEntry[]>([]);
  const [highlightedQueries, setHighlightedQueries] = useState<Map<string, string>>(new Map());
  const [isLoading, setIsLoading] = useState(false);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (open && activeConnection) {
      loadHistory();
    }
  }, [open, activeConnection]);

  const loadHistory = async () => {
    if (!activeConnection) return;

    setIsLoading(true);
    try {
      const entries = await invoke<QueryHistoryEntry[]>("get_query_history", {
        connectionId: activeConnection.id,
      });
      setHistory(entries);

      // Highlight all queries
      const highlighted = new Map<string, string>();

      await Promise.all(
        entries.map(async (entry) => {
          const html = await highlightSQL(entry.query, { schema, keywords });
          highlighted.set(entry.id, html);
        })
      );

      setHighlightedQueries(highlighted);
    } catch (error) {
      ErrorHandler.handle(error, "Failed to load query history");
    } finally {
      setIsLoading(false);
    }
  };

  const handleSelectQuery = (query: string) => {
    onSelectQuery(query);
    setOpen(false);
  };

  const handleDeleteQuery = async (queryId: string) => {
    try {
      await invoke("delete_query_from_history", { queryId });

      // Update local state
      setHistory((prev) => prev.filter((entry) => entry.id !== queryId));
      setHighlightedQueries((prev) => {
        const updated = new Map(prev);
        updated.delete(queryId);
        return updated;
      });
    } catch (error) {
      ErrorHandler.handle(error, "Failed to delete query");
    }
  };

  const handleClearHistory = async () => {
    try {
      await invoke("clear_query_history");
      setHistory([]);
      setHighlightedQueries(new Map());
      ErrorHandler.success("History cleared", "Query history has been cleared");
    } catch (error) {
      ErrorHandler.handle(error, "Failed to clear history");
    }
  };

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return "Just now";
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button
          variant="ghost"
          size={"icon"}
          disabled={!activeConnection}
          title="Query History"
          type="button"
        >
          <Clock className="h-4 w-4" />
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-3xl max-h-[80vh] p-0">
        <DialogHeader className="p-6 pb-4">
          <div className="flex items-center justify-between">
            <div>
              <DialogTitle>Query History</DialogTitle>
              <DialogDescription>
                Recent queries for {activeConnection?.name}
              </DialogDescription>
            </div>
            {history.length > 0 && (
              <AlertDialog>
                <AlertDialogTrigger asChild>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="text-destructive hover:text-destructive"
                  >
                    <Trash2 className="h-4 w-4" />
                    Clear
                  </Button>
                </AlertDialogTrigger>
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle>Clear query history?</AlertDialogTitle>
                    <AlertDialogDescription>
                      This will permanently delete all {history.length} queries from your history.
                      This action cannot be undone.
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  <AlertDialogFooter>
                    <AlertDialogCancel>Cancel</AlertDialogCancel>
                    <AlertDialogAction
                      onClick={handleClearHistory}
                      className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                    >
                      Clear History
                    </AlertDialogAction>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            )}
          </div>
        </DialogHeader>

        <ScrollArea className="h-[500px] px-6 pb-6">
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <p className="text-sm text-muted-foreground">Loading history...</p>
            </div>
          ) : history.length === 0 ? (
            <div className="min-h-[500px] flex items-center justify-center">
              <Empty className="border-0">
                <EmptyHeader>
                  <EmptyMedia variant="icon">
                    <Clock />
                  </EmptyMedia>
                  <EmptyTitle>No Query History</EmptyTitle>
                  <EmptyDescription>
                    Your query history will appear here once you start executing queries.
                  </EmptyDescription>
                </EmptyHeader>
              </Empty>
            </div>
          ) : (
            <div className="space-y-2">
              {history.map((entry) => (
                <div
                  key={entry.id}
                  className="relative rounded-lg border hover:bg-accent transition-colors group"
                >
                  <button
                    onClick={() => handleSelectQuery(entry.query)}
                    className="w-full text-left p-3"
                  >
                    <div className="flex items-start justify-between gap-3 mb-2">
                      <div className="flex items-center gap-2 min-w-0 flex-1">
                        {entry.success ? (
                          <Check className="h-4 w-4 text-green-500 flex-shrink-0" />
                        ) : (
                          <X className="h-4 w-4 text-destructive flex-shrink-0" />
                        )}
                        <span className="text-xs text-muted-foreground truncate">
                          {formatDate(entry.executed_at)}
                        </span>
                        <Badge variant="secondary" className="text-xs">
                          {entry.execution_time_ms.toFixed(0)}ms
                        </Badge>
                      </div>
                    </div>
                    <div
                      className="text-sm font-mono bg-muted p-2 rounded overflow-x-auto whitespace-pre"
                      dangerouslySetInnerHTML={{
                        __html: highlightedQueries.get(entry.id) || entry.query,
                      }}
                    />
                  </button>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDeleteQuery(entry.id);
                    }}
                    className="absolute top-2 right-2 h-7 w-7 opacity-0 group-hover:opacity-100 transition-opacity text-muted-foreground hover:text-destructive"
                    title="Delete query"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>
              ))}
            </div>
          )}
        </ScrollArea>
      </DialogContent>
    </Dialog>
  );
}
