import { useEffect } from "react";
import { Bot, ChevronLeft, ChevronRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { useAiStore } from "@/stores/aiStore";
import { useConnectionStore } from "@/stores/connectionStore";
import { AiModeSelector } from "./AiModeSelector";
import { ChatMessages } from "./ChatMessages";
import { AiChatInput } from "./AiChatInput";
import { QuickActionsMenu } from "./QuickActionsMenu";
import { EmptyState } from "./EmptyState";

export function AiAssistantPanel() {
  const { session, isPanelOpen, initializeSession, setPanelOpen } = useAiStore();
  const { activeConnection } = useConnectionStore();

  // Initialize session when connection changes
  useEffect(() => {
    if (activeConnection && !session) {
      initializeSession(activeConnection.id);
    }
  }, [activeConnection, session, initializeSession]);

  // Collapsed view
  if (!isPanelOpen) {
    return (
      <div className="flex flex-col items-center h-full border-l bg-card p-2">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => setPanelOpen(true)}
          className="mb-4"
        >
          <ChevronLeft className="h-4 w-4" />
        </Button>
        <div className="writing-mode-vertical text-sm font-medium text-muted-foreground">
          AI Assistant
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full border-l bg-card">
      {/* Header */}
      <div className="p-4 border-b">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <div className="rounded-full bg-primary/10 p-2">
              <Bot className="h-4 w-4 text-primary" />
            </div>
            <h2 className="font-semibold">AI Assistant</h2>
          </div>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setPanelOpen(false)}
          >
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>

        {/* AI Mode Selector */}
        <AiModeSelector />
      </div>

      {/* Chat Area */}
      {session && session.messages.length > 0 ? (
        <>
          <ScrollArea className="flex-1 p-4">
            <ChatMessages messages={session.messages} />
          </ScrollArea>

          <Separator />

          {/* Input Area */}
          <div className="p-4 space-y-3">
            <AiChatInput />
            <QuickActionsMenu />
          </div>
        </>
      ) : (
        <EmptyState />
      )}
    </div>
  );
}
