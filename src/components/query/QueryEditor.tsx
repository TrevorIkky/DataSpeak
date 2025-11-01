import { useState } from "react";
import { Play, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { useQueryStore } from "@/stores/queryStore";
import { useConnectionStore } from "@/stores/connectionStore";

export function QueryEditor() {
  const { activeConnection } = useConnectionStore();
  const { tabs, activeTabId, updateTabQuery, executeQuery } = useQueryStore();

  const activeTab = tabs.find((t) => t.id === activeTabId);
  const queryTab = activeTab?.type === 'query' ? activeTab : null;
  const [query, setQuery] = useState(queryTab?.query || "");

  const handleQueryChange = (value: string) => {
    setQuery(value);
    if (activeTabId) {
      updateTabQuery(activeTabId, value);
    }
  };

  const handleExecute = async () => {
    if (!activeTabId || !activeConnection) return;
    await executeQuery(activeTabId, activeConnection.id);
  };

  const isExecuting = queryTab?.isLoading || false;

  return (
    <div className="flex flex-col h-full border-b">
      <div className="flex items-center justify-between px-4 py-2 border-b bg-card">
        <h3 className="text-sm font-semibold">Query Editor</h3>
        <Button
          size="sm"
          onClick={handleExecute}
          disabled={!activeConnection || !query.trim() || isExecuting}
        >
          {isExecuting ? (
            <>
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              Executing...
            </>
          ) : (
            <>
              <Play className="h-4 w-4 mr-2" />
              Execute
            </>
          )}
        </Button>
      </div>

      <div className="flex-1 p-4">
        <Textarea
          value={query}
          onChange={(e) => handleQueryChange(e.target.value)}
          placeholder="Enter your SQL query here...&#10;&#10;Example: SELECT * FROM users WHERE active = true"
          className="h-full font-mono text-sm resize-none"
          disabled={!activeConnection}
        />
      </div>
    </div>
  );
}
