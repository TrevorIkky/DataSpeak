import { useEffect } from "react";
import {
  PlusCircle,
  MessageCircle,
  Trash2,
  PanelLeftClose,
  PanelLeft,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Skeleton } from "@/components/ui/skeleton";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { useAiStore } from "@/stores/aiStore";
import { useConnectionStore } from "@/stores/connectionStore";
import { useIsMobile } from "@/hooks/use-mobile";
import { cn } from "@/lib/utils";
import type { ConversationMetadata } from "@/types/ai.types";

function ConversationItem({
  conversation,
  isActive,
  onSelect,
  onDelete,
}: {
  conversation: ConversationMetadata;
  isActive: boolean;
  onSelect: () => void;
  onDelete: () => void;
}) {
  return (
    <div
      className={cn(
        "group flex items-center gap-1.5 px-2 py-2 rounded-lg cursor-pointer transition-colors overflow-hidden",
        isActive
          ? "bg-primary/10 text-primary border border-primary/20"
          : "hover:bg-accent text-foreground"
      )}
      onClick={onSelect}
    >
      <p className="flex-1 min-w-0 text-sm font-medium truncate leading-tight">
        {conversation.title}
      </p>
      <Button
        variant="ghost"
        size="icon"
        className="opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity h-6 w-6 flex-shrink-0"
        onClick={(e) => {
          e.stopPropagation();
          onDelete();
        }}
      >
        <Trash2 className="h-3 w-3 text-muted-foreground hover:text-destructive" />
      </Button>
    </div>
  );
}

function ConversationList({
  onConversationSelect,
}: {
  onConversationSelect?: () => void;
}) {
  const {
    conversations,
    isLoadingConversations,
    session,
    switchConversation,
    startNewConversation,
    setDeleteConfirmationId,
  } = useAiStore();

  const handleSelect = (sessionId: string) => {
    switchConversation(sessionId);
    onConversationSelect?.();
  };

  const handleNewConversation = () => {
    startNewConversation();
    onConversationSelect?.();
  };

  if (isLoadingConversations) {
    return (
      <div className="p-2 space-y-1 w-full overflow-hidden">
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={i} className="px-2 py-2">
            <Skeleton className="h-4 w-3/4 mb-2" />
            <Skeleton className="h-3 w-1/2" />
          </div>
        ))}
      </div>
    );
  }

  if (conversations.length === 0) {
    return (
      <div className="px-3 py-8 text-center w-full overflow-hidden">
        <MessageCircle className="h-10 w-10 mx-auto text-muted-foreground/50 mb-3" />
        <p className="text-sm font-medium text-muted-foreground">
          No conversations yet
        </p>
        <p className="text-xs text-muted-foreground/70 mt-1">
          Start chatting to create your first conversation
        </p>
        <Button
          variant="outline"
          size="sm"
          className="mt-4"
          onClick={handleNewConversation}
        >
          <PlusCircle className="h-4 w-4 mr-2" />
          New conversation
        </Button>
      </div>
    );
  }

  return (
    <div className="p-2 space-y-1 w-full overflow-hidden">
      {conversations.map((conv) => (
        <ConversationItem
          key={conv.session_id}
          conversation={conv}
          isActive={session?.id === conv.session_id}
          onSelect={() => handleSelect(conv.session_id)}
          onDelete={() => setDeleteConfirmationId(conv.session_id)}
        />
      ))}
    </div>
  );
}

function DeleteConfirmationDialog() {
  const { deleteConfirmationId, deleteConversation, setDeleteConfirmationId } =
    useAiStore();

  return (
    <AlertDialog
      open={deleteConfirmationId !== null}
      onOpenChange={(open) => !open && setDeleteConfirmationId(null)}
    >
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete Conversation</AlertDialogTitle>
          <AlertDialogDescription>
            Are you sure you want to delete this conversation? This action
            cannot be undone.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={() =>
              deleteConfirmationId && deleteConversation(deleteConfirmationId)
            }
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
          >
            Delete
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

export function ConversationSidebar() {
  const {
    sidebarOpen,
    startNewConversation,
    setSidebarOpen,
    loadConversations,
  } = useAiStore();
  const { activeConnection } = useConnectionStore();
  const isMobile = useIsMobile();

  // Load conversations when connection changes
  useEffect(() => {
    if (activeConnection) {
      loadConversations(activeConnection.id);
    }
  }, [activeConnection, loadConversations]);

  // Mobile: Use Sheet (drawer)
  if (isMobile) {
    return (
      <>
        <Sheet open={sidebarOpen} onOpenChange={setSidebarOpen}>
          <SheetContent side="left" className="w-[300px] p-0 flex flex-col">
            <SheetHeader className="px-4 py-3 border-b">
              <div className="flex items-center justify-between">
                <SheetTitle className="text-sm">Conversations</SheetTitle>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8"
                  onClick={() => {
                    startNewConversation();
                    setSidebarOpen(false);
                  }}
                >
                  <PlusCircle className="h-4 w-4" />
                </Button>
              </div>
            </SheetHeader>
            <ScrollArea className="flex-1">
              <ConversationList onConversationSelect={() => setSidebarOpen(false)} />
            </ScrollArea>
          </SheetContent>
        </Sheet>
        <DeleteConfirmationDialog />
      </>
    );
  }

  // Desktop: Collapsed state - slim icon bar
  if (!sidebarOpen) {
    return (
      <>
        <div className="flex flex-col items-center py-3 px-1.5 border-r bg-card gap-2 safe-area-bottom">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8"
                onClick={() => setSidebarOpen(true)}
              >
                <PanelLeft className="h-4 w-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="right">Expand sidebar</TooltipContent>
          </Tooltip>
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
            <TooltipContent side="right">New conversation</TooltipContent>
          </Tooltip>
        </div>
        <DeleteConfirmationDialog />
      </>
    );
  }

  // Desktop: Expanded state
  return (
    <>
      <div className="flex flex-col w-full h-full min-w-0 overflow-hidden border-r bg-card">
        {/* Header */}
        <div className="flex items-center justify-between px-3 py-3 border-b shrink-0">
          <h3 className="font-semibold text-sm truncate">Conversations</h3>
          <div className="flex items-center gap-1 shrink-0">
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
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={() => setSidebarOpen(false)}
            >
              <PanelLeftClose className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* Conversation List */}
        <ScrollArea className="flex-1 w-full">
          <div className="w-full overflow-hidden safe-area-bottom">
            <ConversationList />
          </div>
        </ScrollArea>
      </div>
      <DeleteConfirmationDialog />
    </>
  );
}
