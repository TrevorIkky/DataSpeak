import { useEffect, useRef } from "react";
import { useAiStore } from "@/stores/aiStore";
import { useConnectionStore } from "@/stores/connectionStore";
import { ChatMessages } from "./ChatMessages";
import { AiChatInput } from "./AiChatInput";
import { EmptyState } from "./EmptyState";
import { ConversationSidebar } from "./ConversationSidebar";
import { AlertCircle, X, PanelLeft, PlusCircle } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { TooltipProvider, Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import {
  ResizablePanelGroup,
  ResizablePanel,
  ResizableHandle,
} from "@/components/ui/resizable";
import { useIsMobile } from "@/hooks/use-mobile";

export function AiChatTab() {
  const {
    session,
    isGenerating,
    error,
    sidebarOpen,
    initializeSession,
    loadConversations,
    setSidebarOpen,
    startNewConversation,
  } = useAiStore();
  const { activeConnection } = useConnectionStore();
  const scrollRef = useRef<HTMLDivElement>(null);
  const isMobile = useIsMobile();

  const clearError = () => {
    useAiStore.setState({ error: null });
  };

  // Initialize session and load conversations when connection changes
  useEffect(() => {
    if (activeConnection) {
      if (!session) {
        initializeSession(activeConnection.id);
      }
      loadConversations(activeConnection.id);
    }
  }, [activeConnection, session, initializeSession, loadConversations]);

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

  // Mobile layout
  if (isMobile) {
    return (
      <TooltipProvider>
        <div className="flex flex-col h-full bg-background">
          {/* Mobile Sidebar (Sheet) */}
          <ConversationSidebar />

          {/* Mobile Header */}
          <div className="flex items-center justify-between px-4 py-2 border-b bg-card">
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={() => setSidebarOpen(true)}
            >
              <PanelLeft className="h-4 w-4" />
            </Button>
            <span className="text-sm font-medium">AI Assistant</span>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8"
                  onClick={startNewConversation}
                >
                  <PlusCircle className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>New conversation</TooltipContent>
            </Tooltip>
          </div>

          {/* Messages Area */}
          <div className="flex-1 overflow-hidden" ref={scrollRef}>
            {!session || session.messages.length === 0 ? (
              <EmptyState />
            ) : (
              <div className="h-full overflow-y-auto p-4">
                <ChatMessages
                  messages={session.messages}
                  isGenerating={isGenerating}
                />
              </div>
            )}
          </div>

          {/* Error Display */}
          {error && (
            <div className="border-t bg-card p-4">
              <Alert variant="destructive" className="relative">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription className="pr-8">{error}</AlertDescription>
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
          <div className="border-t bg-card p-4 safe-area-bottom">
            <AiChatInput />
          </div>
        </div>
      </TooltipProvider>
    );
  }

  // Desktop layout with resizable panels
  return (
    <TooltipProvider>
      <ResizablePanelGroup direction="horizontal" className="h-full">
        {/* Conversation Sidebar */}
        {sidebarOpen && (
          <>
            <ResizablePanel
              defaultSize={24}
              minSize={22}
              maxSize={28}
            >
              <ConversationSidebar />
            </ResizablePanel>
            <ResizableHandle withHandle />
          </>
        )}

        {/* Collapsed Sidebar */}
        {!sidebarOpen && <ConversationSidebar />}

        {/* Main Chat Area */}
        <ResizablePanel defaultSize={sidebarOpen ? 76 : 100} minSize={50}>
          <div className="flex flex-col h-full bg-background">
            {/* Messages Area */}
            <div className="flex-1 overflow-hidden" ref={scrollRef}>
              {!session || session.messages.length === 0 ? (
                <EmptyState />
              ) : (
                <div className="h-full overflow-y-auto p-4">
                  <ChatMessages
                    messages={session.messages}
                    isGenerating={isGenerating}
                  />
                </div>
              )}
            </div>

            {/* Error Display */}
            {error && (
              <div className="border-t bg-card p-4">
                <Alert variant="destructive" className="relative">
                  <AlertCircle className="h-4 w-4" />
                  <AlertDescription className="pr-8">{error}</AlertDescription>
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
            <div className="border-t bg-card p-4 safe-area-bottom">
              <AiChatInput />
            </div>
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </TooltipProvider>
  );
}
