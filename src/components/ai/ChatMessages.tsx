import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import type { ChatMessage } from "@/types/ai.types";
import ReactMarkdown from "react-markdown";
import { MessageContent } from "./MessageContent";

interface ChatMessagesProps {
  messages: ChatMessage[];
}

export function ChatMessages({ messages }: ChatMessagesProps) {
  return (
    <div className="space-y-4">
      {messages.map((message) => (
        <div
          key={message.id}
          className={cn(
            "flex flex-col",
            message.role === 'user' ? "items-end" : "items-start"
          )}
        >
          {/* Message bubble */}
          <Card className={cn(
            "shadow-sm",
            message.role === 'user'
              ? "p-3 max-w-[75%] bg-primary text-primary-foreground border-primary"
              : "p-4 max-w-[85%] bg-muted border-border"
          )}>
            {/* User messages: simple text */}
            {message.role === 'user' && (
              <div className="prose prose-sm max-w-none prose-invert [&>p]:text-primary-foreground [&>*]:text-primary-foreground">
                <ReactMarkdown>{message.content}</ReactMarkdown>
              </div>
            )}

            {/* Assistant messages: rich content with inline rendering */}
            {message.role === 'assistant' && (
              <MessageContent message={message} />
            )}

            {/* Message metadata */}
            {message.metadata && (
              <div className={cn(
                "mt-2 pt-2 border-t text-xs",
                message.role === 'user'
                  ? "border-primary-foreground/20 text-primary-foreground/80"
                  : "border-border text-muted-foreground"
              )}>
                {message.metadata.executionTime && (
                  <span>Executed in {message.metadata.executionTime}ms</span>
                )}
                {message.metadata.affectedRows !== undefined && (
                  <span className="ml-2">
                    {message.metadata.affectedRows} rows affected
                  </span>
                )}
              </div>
            )}

            {/* TODO: Add MessageActions component here if message.actions exists */}
          </Card>

          {/* Timestamp below the bubble */}
          <div className="flex items-center gap-2 mt-1 px-1">
            <span className="text-xs font-medium text-muted-foreground">
              {message.role === 'assistant' ? 'AI Assistant' : 'You'}
            </span>
            <span className="text-xs text-muted-foreground">
              {new Date(message.timestamp).toLocaleTimeString([], {
                hour: '2-digit',
                minute: '2-digit',
              })}
            </span>
          </div>
        </div>
      ))}
    </div>
  );
}
