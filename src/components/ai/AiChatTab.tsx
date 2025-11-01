import { useEffect, useRef } from "react";
import { useAiStore } from "@/stores/aiStore";
import { useConnectionStore } from "@/stores/connectionStore";
import { ChatMessages } from "./ChatMessages";
import { AiChatInput } from "./AiChatInput";
import { EmptyState } from "./EmptyState";
import { Loader2, AlertCircle, X } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";

export function AiChatTab() {
  const { session, isGenerating, error, initializeSession } = useAiStore();
  const { activeConnection } = useConnectionStore();
  const scrollRef = useRef<HTMLDivElement>(null);

  const clearError = () => {
    useAiStore.setState({ error: null });
  };

  // Initialize session when connection changes
  useEffect(() => {
    if (activeConnection && !session) {
      initializeSession(activeConnection.id);
    }
  }, [activeConnection, session, initializeSession]);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [session?.messages]);

  if (!activeConnection) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-8">
        <p className="text-sm text-muted-foreground text-center max-w-md">
          Connect to a database to start chatting with AI
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-background">
      {/* Messages Area */}
      <div className="flex-1 overflow-hidden" ref={scrollRef}>
        {!session || session.messages.length === 0 ? (
          <EmptyState />
        ) : (
          <div className="h-full overflow-y-auto p-4">
            <ChatMessages messages={session.messages} />
            {isGenerating && (
              <div className="flex items-center gap-2 text-sm text-muted-foreground mt-4">
                <Loader2 className="h-4 w-4 animate-spin" />
                <span>AI is thinking...</span>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Error Display */}
      {error && (
        <div className="border-t bg-card p-4">
          <Alert variant="destructive" className="relative">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription className="pr-8">
              {error}
            </AlertDescription>
            <Button
              variant="ghost"
              size="icon"
              className="absolute right-2 top-2 h-6 w-6"
              onClick={clearError}
            >
              <X className="h-4 w-4" />
            </Button>
          </Alert>
        </div>
      )}

      {/* Input Area */}
      <div className="border-t bg-card p-4">
        <AiChatInput />
      </div>
    </div>
  );
}
