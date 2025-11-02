import { useState } from "react";
import { Play, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { SqlAutocompleteTextarea } from "./SqlAutocompleteTextarea";
import { useQueryStore } from "@/stores/queryStore";
import { useConnectionStore } from "@/stores/connectionStore";
import { useSchemaStore } from "@/stores/schemaStore";

export function QueryEditor() {
  const { activeConnection } = useConnectionStore();
  const { tabs, activeTabId, updateTabQuery, executeQuery } = useQueryStore();
  const { schema, keywords, fetchKeywords } = useSchemaStore();

  const activeTab = tabs.find((t) => t.id === activeTabId);
  const queryTab = activeTab?.type === 'query' ? activeTab : null;
  const [query, setQuery] = useState(queryTab?.query || "");

  // Fetch keywords when user starts typing
  const handleFirstKeystroke = () => {
    if (activeConnection?.id && keywords.length === 0) {
      fetchKeywords(activeConnection.id);
    }
  };

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
        <SqlAutocompleteTextarea
          value={query}
          onChange={handleQueryChange}
          placeholder="Enter your SQL query here...&#10;&#10;Example: SELECT * FROM users WHERE active = true"
          className="h-full font-mono text-sm resize-none"
          disabled={!activeConnection}
          schema={schema}
          keywords={keywords}
          onFirstKeystroke={handleFirstKeystroke}
        />
      </div>
    </div>
  );
}
