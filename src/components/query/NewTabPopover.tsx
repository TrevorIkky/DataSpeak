import { useState } from "react";
import { Plus, Code, MessageCircleMore } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { useQueryStore } from "@/stores/queryStore";

interface NewTabPopoverProps {
  /** Custom trigger element. If not provided, uses default Button */
  trigger?: React.ReactNode;
  /** Popover alignment */
  align?: "start" | "center" | "end";
  /** Whether to hide the AI Assistant option */
  hideChatOption?: boolean;
  /** Additional class for the trigger */
  triggerClassName?: string;
}

export function NewTabPopover({
  trigger,
  align = "center",
  hideChatOption = false,
  triggerClassName,
}: NewTabPopoverProps) {
  const [open, setOpen] = useState(false);
  const { addTab, addChatTab } = useQueryStore();
  const hasChatTab = useQueryStore((state) => state.tabs.some((t) => t.type === "chat"));

  const showChatOption = !hideChatOption && !hasChatTab;

  const handleAddQuery = () => {
    addTab();
    setOpen(false);
  };

  const handleAddChat = () => {
    addChatTab();
    setOpen(false);
  };

  // If chat option is hidden or already exists, just add query directly
  if (!showChatOption) {
    return trigger ? (
      <div onClick={handleAddQuery} className={triggerClassName}>
        {trigger}
      </div>
    ) : (
      <Button onClick={handleAddQuery} className={triggerClassName}>
        <Plus className="h-4 w-4" />
        New Query
      </Button>
    );
  }

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        {trigger || (
          <Button className={triggerClassName}>
            <Plus className="h-4 w-4" />
            New Tab
          </Button>
        )}
      </PopoverTrigger>
      <PopoverContent className="w-64 p-2" align={align} side="bottom" sideOffset={8}>
        <div className="space-y-1">
          <button
            className="w-full flex items-start gap-2 px-2 py-2 hover:bg-accent rounded-md transition-colors text-left"
            onClick={handleAddQuery}
          >
            <Code className="h-4 w-4 mt-0.5 shrink-0" />
            <div>
              <div className="font-medium text-sm">New SQL Query</div>
              <div className="text-xs text-muted-foreground">Write and execute SQL</div>
            </div>
          </button>
          <button
            className="w-full flex items-start gap-2 px-2 py-2 hover:bg-accent rounded-md transition-colors text-left"
            onClick={handleAddChat}
          >
            <MessageCircleMore className="h-4 w-4 mt-0.5 shrink-0" />
            <div>
              <div className="font-medium text-sm">AI Assistant</div>
              <div className="text-xs text-muted-foreground">Chat with AI about your data</div>
            </div>
          </button>
        </div>
      </PopoverContent>
    </Popover>
  );
}
