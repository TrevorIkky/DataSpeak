import { Sparkles, TrendingUp, MessageSquare, Zap, AlertCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useAiStore } from "@/stores/aiStore";
import { useQueryStore } from "@/stores/queryStore";

export function QuickActionsMenu() {
  const { sendMessage, setMode } = useAiStore();
  const { tabs, activeTabId } = useQueryStore();

  const activeTab = tabs.find((t) => t.id === activeTabId);
  const hasQuery = activeTab?.type === 'query' && activeTab.query;
  const hasResults = activeTab?.type === 'query' && activeTab.result;

  const handleAnalyzeData = () => {
    if (!hasResults) return;
    setMode('analyst');
    sendMessage("Analyze the current query results and provide insights.");
  };

  const handleExplainQuery = () => {
    if (!hasQuery || activeTab?.type !== 'query') return;
    setMode('explain');
    sendMessage(`Explain this SQL query:\n\`\`\`sql\n${activeTab.query}\n\`\`\``);
  };

  const handleOptimize = () => {
    if (!hasQuery || activeTab?.type !== 'query') return;
    setMode('sql');
    sendMessage(`Suggest optimizations for this query:\n\`\`\`sql\n${activeTab.query}\n\`\`\``);
  };

  const handleCheckQuality = () => {
    if (!hasResults) return;
    setMode('quality');
    sendMessage("Check the data quality of the current results.");
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" className="w-full">
          <Sparkles className="h-4 w-4 mr-2" />
          Quick Actions
        </Button>
      </DropdownMenuTrigger>

      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuItem
          onClick={handleAnalyzeData}
          disabled={!hasResults}
        >
          <TrendingUp className="mr-2 h-4 w-4" />
          Analyze visible data
        </DropdownMenuItem>

        <DropdownMenuItem
          onClick={handleExplainQuery}
          disabled={!hasQuery}
        >
          <MessageSquare className="mr-2 h-4 w-4" />
          Explain current query
        </DropdownMenuItem>

        <DropdownMenuItem
          onClick={handleOptimize}
          disabled={!hasQuery}
        >
          <Zap className="mr-2 h-4 w-4" />
          Suggest optimizations
        </DropdownMenuItem>

        <DropdownMenuItem
          onClick={handleCheckQuality}
          disabled={!hasResults}
        >
          <AlertCircle className="mr-2 h-4 w-4" />
          Check data quality
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
