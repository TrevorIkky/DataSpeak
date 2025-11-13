import { useState, useEffect, useRef } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Check, X, Sparkles, Send, ChevronDown, ChevronRight } from "lucide-react";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";

interface AiQueryFloatingWindowProps {
  position: { x: number; y: number };
  onClose: () => void;
  onSubmit: (prompt: string) => void;
  onApprove: () => void;
  onReject: () => void;
  isGenerating: boolean;
  isComplete: boolean;
  thinkingContent?: string;
  error?: string | null;
}

export function AiQueryFloatingWindow({
  position,
  onClose,
  onSubmit,
  onApprove,
  onReject,
  isGenerating,
  isComplete,
  thinkingContent = "",
  error = null,
}: AiQueryFloatingWindowProps) {
  const [prompt, setPrompt] = useState("");
  const [phase, setPhase] = useState<"input" | "thinking" | "generated" | "error">("input");
  const [thinkingOpen, setThinkingOpen] = useState(false);
  const cardRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Focus input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Update phase based on generating state and errors
  useEffect(() => {
    if (error) {
      setPhase("error");
    } else if (phase === "input" && isGenerating) {
      setPhase("thinking");
    } else if (isComplete && phase !== "input") {
      // Transition to generated as soon as SQL is ready, even if still generating
      setPhase("generated");
    }
  }, [isGenerating, isComplete, phase, error]);

  // Handle escape key
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (phase === "thinking" && !isComplete) {
          // Don't close during generation
          return;
        }
        onClose();
      }
    };

    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [onClose, phase, isComplete]);

  // Handle click outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (cardRef.current && !cardRef.current.contains(e.target as Node)) {
        if (phase === "thinking" && !isComplete) {
          // Don't close during generation
          return;
        }
        onClose();
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [onClose, phase, isComplete]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (prompt.trim()) {
      onSubmit(prompt);
      setPhase("thinking");
    }
  };

  const handleApprove = () => {
    onApprove();
    onClose();
  };

  const handleReject = () => {
    onReject();
    onClose();
  };

  return (
    <div
      ref={cardRef}
      className="fixed z-50"
      style={{
        left: `${position.x}px`,
        top: `${position.y}px`,
      }}
    >
      <Card className="w-96 shadow-lg border-2 animate-in fade-in-0 zoom-in-95 duration-200 py-0 gap-0">
        <CardContent className="p-4 space-y-2">
          {phase === "input" && (
            <>
              <div className="flex items-center gap-2 mb-2">
                <Sparkles className="h-4 w-4 text-primary" />
                <h3 className="font-semibold text-sm">Ask AI to Generate Query</h3>
              </div>
              <form onSubmit={handleSubmit}>
                <div className="relative">
                  <Input
                    ref={inputRef}
                    value={prompt}
                    onChange={(e) => setPrompt(e.target.value)}
                    placeholder="E.g., show me top 10 customers by revenue"
                    className="w-full pr-10"
                    autoComplete="off"
                  />
                  <button
                    type="submit"
                    disabled={!prompt.trim()}
                    className={cn(
                      "absolute right-2 top-1/2 -translate-y-1/2 p-1.5 rounded-md transition-colors",
                      prompt.trim()
                        ? "text-primary hover:bg-accent cursor-pointer"
                        : "text-muted-foreground cursor-not-allowed opacity-50"
                    )}
                  >
                    <Send className="h-4 w-4" />
                  </button>
                </div>
              </form>
            </>
          )}

          {phase === "thinking" && (
            <>
              <div className="flex items-center gap-2 mb-1">
                <Sparkles className="h-4 w-4 text-primary animate-pulse" />
                <h3 className="font-semibold text-sm">Thinking...</h3>
              </div>

              {/* Show streaming thinking content */}
              {thinkingContent ? (
                <div className="text-xs text-muted-foreground p-2 bg-muted/50 rounded border border-border max-h-32 overflow-y-auto">
                  {thinkingContent}
                </div>
              ) : (
                /* Shimmer effect if no content yet */
                <div className="space-y-2 py-1">
                  <div className="h-4 bg-gradient-to-r from-muted via-muted-foreground/20 to-muted rounded animate-shimmer bg-[length:200%_100%]" />
                  <div className="h-4 bg-gradient-to-r from-muted via-muted-foreground/20 to-muted rounded animate-shimmer bg-[length:200%_100%] w-4/5" style={{ animationDelay: '0.1s' }} />
                  <div className="h-4 bg-gradient-to-r from-muted via-muted-foreground/20 to-muted rounded animate-shimmer bg-[length:200%_100%] w-3/5" style={{ animationDelay: '0.2s' }} />
                </div>
              )}

              {/* Action buttons - disabled during thinking */}
              <div className="flex justify-end gap-2 pt-1">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleReject}
                  disabled={true}
                >
                  <X className="h-4 w-4 mr-1" />
                  Reject
                </Button>
                <Button
                  size="sm"
                  onClick={handleApprove}
                  disabled={true}
                >
                  <Check className="h-4 w-4 mr-1" />
                  Approve
                </Button>
              </div>
            </>
          )}

          {phase === "generated" && (
            <>
              <div className="flex items-center gap-2 mb-1">
                <Sparkles className="h-4 w-4 text-primary" />
                <h3 className="font-semibold text-sm">Query Generated</h3>
              </div>

              {/* Thinking content accordion */}
              {thinkingContent && (
                <Collapsible open={thinkingOpen} onOpenChange={setThinkingOpen}>
                  <CollapsibleTrigger className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors">
                    {thinkingOpen ? (
                      <ChevronDown className="h-3 w-3" />
                    ) : (
                      <ChevronRight className="h-3 w-3" />
                    )}
                    <span>Show thinking process</span>
                  </CollapsibleTrigger>
                  <CollapsibleContent className="mt-2">
                    <div className="text-xs text-muted-foreground p-2 bg-muted/50 rounded border border-border max-h-32 overflow-y-auto">
                      {thinkingContent}
                    </div>
                  </CollapsibleContent>
                </Collapsible>
              )}

              {/* Action buttons */}
              <div className="flex justify-end gap-2 pt-1">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleReject}
                >
                  <X className="h-4 w-4 mr-1" />
                  Reject
                </Button>
                <Button
                  size="sm"
                  onClick={handleApprove}
                >
                  <Check className="h-4 w-4 mr-1" />
                  Approve
                </Button>
              </div>
            </>
          )}

          {phase === "error" && (
            <>
              <div className="flex items-center gap-2 mb-2">
                <X className="h-4 w-4 text-destructive" />
                <h3 className="font-semibold text-sm text-destructive">Error</h3>
              </div>

              {/* Error message */}
              <div className="text-sm text-destructive p-3 bg-destructive/10 rounded border border-destructive/20">
                {error || "An unexpected error occurred"}
              </div>

              {/* Close button */}
              <div className="flex justify-end pt-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={onClose}
                >
                  Close
                </Button>
              </div>
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
