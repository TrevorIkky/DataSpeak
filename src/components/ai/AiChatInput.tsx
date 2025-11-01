import { useState } from "react";
import { Send, Loader2 } from "lucide-react";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import { useAiStore } from "@/stores/aiStore";

export function AiChatInput() {
  const [input, setInput] = useState("");
  const { sendMessage, isGenerating } = useAiStore();

  const handleSubmit = async () => {
    if (!input.trim() || isGenerating) return;

    await sendMessage(input.trim());
    setInput("");
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <div className="space-y-2">
      <Textarea
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Ask me anything about your data..."
        className="min-h-[80px] resize-none"
        disabled={isGenerating}
      />

      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground">
          Press <kbd className="px-1.5 py-0.5 text-xs font-semibold text-foreground bg-muted border border-border rounded">Enter</kbd> to send, <kbd className="px-1.5 py-0.5 text-xs font-semibold text-foreground bg-muted border border-border rounded">Shift+Enter</kbd> for new line
        </span>

        <Button
          size="sm"
          onClick={handleSubmit}
          disabled={!input.trim() || isGenerating}
        >
          {isGenerating ? (
            <>
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              Thinking...
            </>
          ) : (
            <>
              <Send className="h-4 w-4 mr-2" />
              Send
            </>
          )}
        </Button>
      </div>
    </div>
  );
}
