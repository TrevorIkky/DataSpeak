import { useState, useEffect, useRef } from "react";
import { Play, Loader2, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { Kbd, KbdGroup } from "@/components/ui/kbd";
import { SqlAutocompleteTextarea } from "./SqlAutocompleteTextarea";
import { QueryHistoryDialog } from "./QueryHistoryDialog";
import { AiQueryFloatingWindow } from "./AiQueryFloatingWindow";
import { useQueryStore } from "@/stores/queryStore";
import { useConnectionStore } from "@/stores/connectionStore";
import { useSchemaStore } from "@/stores/schemaStore";
import { useUIStore } from "@/stores/uiStore";
import { useAiStore } from "@/stores/aiStore";

type ExecutionMode = 'current' | 'all';

export function QueryEditor() {
  const { activeConnection } = useConnectionStore();
  const { tabs, activeTabId, updateTabQuery, executeQuery } = useQueryStore();
  const { schema, keywords, fetchKeywords } = useSchemaStore();
  const {
    executionMode,
    aiQueryWindowOpen,
    aiQueryWindowPosition,
    aiQueryOriginalQuery,
    aiQueryGeneratedSql,
    aiQueryThinkingContent,
    aiQueryError,
    isAiQueryGenerating,
    openAiQueryWindow,
    closeAiQueryWindow,
    startAiQueryGeneration,
    updateAiQueryThinkingContent,
    updateAiQuerySql,
    completeAiQueryGeneration,
    setAiQueryError,
  } = useUIStore();
  const { session, isGenerating, error, initializeSession, sendMessage } = useAiStore();

  const activeTab = tabs.find((t) => t.id === activeTabId);
  const queryTab = activeTab?.type === 'query' ? activeTab : null;
  const [query, setQuery] = useState(queryTab?.query || "");
  const [cursorPosition, setCursorPosition] = useState(0);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  // Sync local query state with active tab
  useEffect(() => {
    setQuery(queryTab?.query || "");
  }, [activeTabId, queryTab?.query]);

  // Fetch keywords when user starts typing
  const handleFirstKeystroke = () => {
    if (activeConnection?.id && keywords.length === 0) {
      fetchKeywords(activeConnection.id);
    }
  };

  // Handle AI errors
  useEffect(() => {
    if (!aiQueryWindowOpen || !error) return;
    setAiQueryError(error);
  }, [error, aiQueryWindowOpen, setAiQueryError]);

  // Handle AI streaming and completion
  useEffect(() => {
    if (!session || !aiQueryWindowOpen || !isAiQueryGenerating || !activeTabId) return;

    const lastMessage = session.messages[session.messages.length - 1];
    if (lastMessage?.role !== 'assistant' || !lastMessage.content) return;

    const content = lastMessage.content;

    // Extract SQL from content (within ```sql...``` blocks)
    const sqlMatch = content.match(/```sql\n([\s\S]*?)\n```/);
    const extractedSql = sqlMatch?.[1]?.trim() || "";

    // Extract thinking content (everything before SQL block, or all content if no SQL block yet)
    let thinkingContent = "";
    if (sqlMatch) {
      // Get everything before the SQL block
      const sqlBlockStart = content.indexOf("```sql");
      thinkingContent = content.substring(0, sqlBlockStart).trim();
    } else {
      // No SQL block yet, all content is thinking
      thinkingContent = content.trim();
    }

    // Update thinking content in store (for accordion display)
    if (thinkingContent) {
      updateAiQueryThinkingContent(thinkingContent);
    }

    // Update editor with SQL as it streams in
    if (extractedSql) {
      const updatedQuery = aiQueryOriginalQuery ? `${aiQueryOriginalQuery}\n\n${extractedSql}` : extractedSql;
      setQuery(updatedQuery);
      updateTabQuery(activeTabId, updatedQuery);

      // Update SQL state immediately - this enables buttons as soon as SQL appears
      updateAiQuerySql(extractedSql);
    }

    // When backend finishes, mark generation as complete
    if (!isGenerating && extractedSql) {
      completeAiQueryGeneration(extractedSql, thinkingContent);
    }
  }, [session?.messages, isGenerating, aiQueryWindowOpen, isAiQueryGenerating, activeTabId]);

  const handleQueryChange = (value: string) => {
    setQuery(value);
    if (activeTabId) {
      updateTabQuery(activeTabId, value);
    }
  };

  const handleInsertQuery = (newQuery: string) => {
    // Insert on a new line at the end
    const trimmedQuery = query.trim();
    const updatedQuery = trimmedQuery
      ? `${trimmedQuery}\n\n${newQuery}`
      : newQuery;

    handleQueryChange(updatedQuery);

    // Focus textarea and move cursor to end
    setTimeout(() => {
      if (textareaRef.current) {
        textareaRef.current.focus();
        textareaRef.current.selectionStart = updatedQuery.length;
        textareaRef.current.selectionEnd = updatedQuery.length;
      }
    }, 0);
  };

  // Extract the current SQL statement at cursor position
  const getCurrentStatement = (text: string, position: number): string => {
    // Split by semicolons to get individual statements
    const statements = text.split(';').map(s => s.trim()).filter(s => s.length > 0);

    if (statements.length === 0) return text.trim();
    if (statements.length === 1) return statements[0];

    // Find which statement contains the cursor
    let currentPos = 0;
    for (const statement of statements) {
      const statementEnd = currentPos + statement.length + 1; // +1 for semicolon
      if (position <= statementEnd) {
        return statement;
      }
      currentPos = statementEnd;
    }

    // If cursor is at the end, return the last statement
    return statements[statements.length - 1];
  };

  const handleExecute = async (mode?: ExecutionMode) => {
    if (!activeTabId || !activeConnection) return;

    const execMode = mode || executionMode;

    if (execMode === 'current') {
      // Execute only the current statement
      const currentStatement = getCurrentStatement(query, cursorPosition);
      if (!currentStatement.trim()) return;

      // Temporarily update the tab with just the current statement
      const originalQuery = queryTab?.query || "";
      updateTabQuery(activeTabId, currentStatement);
      await executeQuery(activeTabId, activeConnection.id);
      // Restore the full query
      updateTabQuery(activeTabId, originalQuery);
    } else {
      // Execute all queries
      await executeQuery(activeTabId, activeConnection.id);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Ctrl/Cmd + Enter: Execute current statement
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleExecute('current');
    }
    // Ctrl/Cmd + Shift + Enter: Execute all
    else if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'Enter') {
      e.preventDefault();
      handleExecute('all');
    }
  };

  const handleCursorChange = (e: React.SyntheticEvent<HTMLTextAreaElement>) => {
    const target = e.target as HTMLTextAreaElement;
    setCursorPosition(target.selectionStart);
  };

  // AI Query Generation Handlers
  const handleOpenAiWindow = (e: React.MouseEvent) => {
    if (!activeConnection) return;

    // Initialize AI session if needed
    if (!session) {
      initializeSession(activeConnection.id);
    }

    openAiQueryWindow({ x: e.clientX, y: e.clientY }, query);
  };

  const handleAiSubmit = async (prompt: string) => {
    if (!activeConnection) return;

    startAiQueryGeneration();

    // Instruct the agent to use execute_sql with dry_run=true
    const enhancedPrompt = `Generate a SQL query for: ${prompt}\n\nIMPORTANT: Use the execute_sql tool with dry_run=true to generate the SQL query without executing it. Just return the SQL code.`;
    await sendMessage(enhancedPrompt);
  };

  const handleAiApprove = () => {
    closeAiQueryWindow();
  };

  const handleAiReject = () => {
    // Restore original query
    setQuery(aiQueryOriginalQuery);
    if (activeTabId) {
      updateTabQuery(activeTabId, aiQueryOriginalQuery);
    }
    closeAiQueryWindow();
  };

  const isExecuting = queryTab?.isLoading || false;

  const executionLabel = executionMode === 'current'
    ? 'Execute Current'
    : 'Execute All';

  return (
    <div className="flex flex-col h-full border-b">
      <div className="flex items-center justify-between px-4 py-2 border-b bg-card">
        <h3 className="text-sm font-semibold">Query Editor</h3>
        <div className="flex items-center gap-2">
          <QueryHistoryDialog onSelectQuery={handleInsertQuery} />

          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                size="sm"
                onClick={() => handleExecute()}
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
                    {executionLabel}
                  </>
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              <KbdGroup>
                <Kbd>⌘</Kbd>
                <Kbd>↵</Kbd>
              </KbdGroup>
              {' or '}
              <KbdGroup>
                <Kbd>Ctrl</Kbd>
                <Kbd>↵</Kbd>
              </KbdGroup>
            </TooltipContent>
          </Tooltip>
        </div>
      </div>

      <div className="flex-1 p-4 safe-area-bottom">
        <ContextMenu>
          <ContextMenuTrigger asChild>
            <div className="h-full">
              <SqlAutocompleteTextarea
                ref={textareaRef}
                value={query}
                onChange={handleQueryChange}
                onKeyDown={handleKeyDown}
                onSelect={handleCursorChange}
                onClick={handleCursorChange}
                placeholder={`Enter your SQL query here...\n\nTip: Use ; to separate multiple statements\n• ⌘↵ (Ctrl+↵) - Execute current statement\n• ⇧⌘↵ (Ctrl+Shift+↵) - Execute all statements`}
                className="h-full font-mono text-sm resize-none"
                disabled={!activeConnection}
                schema={schema}
                keywords={keywords}
                onFirstKeystroke={handleFirstKeystroke}
              />
            </div>
          </ContextMenuTrigger>
          <ContextMenuContent>
            <ContextMenuItem
              onClick={handleOpenAiWindow}
              disabled={!activeConnection}
            >
              <Sparkles className="h-4 w-4 mr-2" />
              Ask AI to Generate Query...
            </ContextMenuItem>
          </ContextMenuContent>
        </ContextMenu>
      </div>

      {/* AI Query Generation Floating Window */}
      {aiQueryWindowOpen && (
        <AiQueryFloatingWindow
          position={aiQueryWindowPosition}
          onClose={closeAiQueryWindow}
          onSubmit={handleAiSubmit}
          onApprove={handleAiApprove}
          onReject={handleAiReject}
          isGenerating={isAiQueryGenerating}
          isComplete={aiQueryGeneratedSql.length > 0}
          thinkingContent={aiQueryThinkingContent}
          error={aiQueryError}
          hasSql={aiQueryGeneratedSql.length > 0}
        />
      )}
    </div>
  );
}
