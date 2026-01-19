import { useEffect, useRef } from "react";
import { cn } from "@/lib/utils";
import type { ChatMessage } from "@/types/ai.types";
import ReactMarkdown from "react-markdown";
import { MessageContent } from "./MessageContent";

interface ChatMessagesProps {
  messages: ChatMessage[];
  isGenerating?: boolean;
}

export function ChatMessages({ messages, isGenerating }: ChatMessagesProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  // Check if the last message is still in thinking state (no content yet)
  const lastMessage = messages[messages.length - 1];
  const isThinking = isGenerating && lastMessage?.role === 'assistant' && !lastMessage?.content && !lastMessage?.thinking;

  // Filter out completely empty assistant messages when thinking
  const displayMessages = isThinking
    ? messages.slice(0, -1)
    : messages;

  // Auto-scroll to bottom when messages change or during generation
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, lastMessage?.content, lastMessage?.thinking]);

  return (
    <div className="space-y-4">
      {displayMessages.map((message) => (
        <div
          key={message.id}
          className={cn(
            "flex flex-col",
            message.role === 'user' ? "items-end" : "items-start"
          )}
        >
          {/* Message bubble */}
          {message.role === 'user' ? (
            <div className="relative max-w-[85%] sm:max-w-[75%] md:max-w-[65%]">
              <div className="px-4 py-2.5 bg-primary text-primary-foreground rounded-2xl rounded-br-none shadow-sm">
                <div className="text-sm leading-relaxed [&>p]:m-0">
                  <ReactMarkdown>{message.content}</ReactMarkdown>
                </div>
              </div>
              {/* iOS-style tail */}
              <div className="absolute bottom-0 right-0 w-3 h-3 bg-primary rounded-bl-full" />
              {/* Message metadata inside bubble */}
              {message.metadata && (
                <div className="mt-1.5 px-1 text-xs text-primary-foreground/70">
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
            </div>
          ) : (
            <div className="w-full">
              <MessageContent message={message} />
              {/* Message metadata */}
              {message.metadata && (
                <div className="mt-2 pt-2 border-t border-border text-xs text-muted-foreground">
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
            </div>
          )}

          {/* Timestamp below the bubble */}
          <div className={cn(
            "flex items-center gap-1.5 mt-1",
            message.role === 'user' ? "pr-1" : "pl-1"
          )}>
            <span className="text-xs text-muted-foreground/70">
              {new Date(message.timestamp).toLocaleTimeString([], {
                hour: '2-digit',
                minute: '2-digit',
              })}
            </span>
          </div>
        </div>
      ))}

      {/* Thinking indicator */}
      {isThinking && (
        <div className="flex flex-col items-start">
          <div className="text-sm text-muted-foreground">
            <span className="font-medium shine-text">AI is thinking...</span>
          </div>
          <div className="flex items-center gap-2 mt-1">
            <span className="text-xs font-medium text-muted-foreground">AI Assistant</span>
            <span className="text-xs text-muted-foreground">
              {new Date().toLocaleTimeString([], {
                hour: '2-digit',
                minute: '2-digit',
              })}
            </span>
          </div>
        </div>
      )}

      {/* Scroll anchor */}
      <div ref={bottomRef} />
    </div>
  );
}
