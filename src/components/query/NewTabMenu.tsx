import { Code, MessageCircleMore } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useQueryStore } from "@/stores/queryStore";

interface NewTabMenuProps {
  onClose?: () => void;
}

export function NewTabMenu({ onClose }: NewTabMenuProps) {
  const { addTab, addChatTab, tabs } = useQueryStore();
  const hasChatTab = tabs.some((t) => t.type === "chat");

  const handleAddQuery = () => {
    addTab();
    onClose?.();
  };

  const handleAddChat = () => {
    addChatTab();
    onClose?.();
  };

  return (
    <div className="space-y-1">
      <Button
        variant="ghost"
        className="w-full justify-start h-auto py-2 px-2"
        onClick={handleAddQuery}
      >
        <Code className="h-4 w-4 mr-2 shrink-0" />
        <div className="text-left">
          <div className="font-medium text-sm">New SQL Query</div>
          <div className="text-xs text-muted-foreground font-normal">Write and execute SQL</div>
        </div>
      </Button>
      {!hasChatTab && (
        <Button
          variant="ghost"
          className="w-full justify-start h-auto py-2 px-2"
          onClick={handleAddChat}
        >
          <MessageCircleMore className="h-4 w-4 mr-2 shrink-0" />
          <div className="text-left">
            <div className="font-medium text-sm">AI Assistant</div>
            <div className="text-xs text-muted-foreground font-normal">Chat with AI about your data</div>
          </div>
        </Button>
      )}
    </div>
  );
}
