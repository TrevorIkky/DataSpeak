import { useState, useRef, useEffect, KeyboardEvent } from "react";
import { Textarea } from "@/components/ui/textarea";
import getCaretCoordinates from "textarea-caret";
import {
  detectContext,
  generateSuggestions,
  insertSuggestion,
  type Suggestion,
} from "@/lib/sqlAutocomplete";
import { highlightSQL } from "@/lib/sqlSyntaxHighlight";
import type { Schema, SqlKeyword } from "@/types/database.types";

interface SqlAutocompleteTextareaProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  schema: Schema | null;
  keywords: SqlKeyword[];
  onFirstKeystroke?: () => void;
}

export function SqlAutocompleteTextarea({
  value,
  onChange,
  placeholder,
  disabled,
  className,
  schema,
  keywords,
  onFirstKeystroke,
}: SqlAutocompleteTextareaProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [cursorPosition, setCursorPosition] = useState(0);
  const [position, setPosition] = useState({ top: 0, left: 0 });
  const [highlightedHTML, setHighlightedHTML] = useState('');
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const highlightRef = useRef<HTMLDivElement>(null);
  const hasTypedRef = useRef(false);
  const lastInsertedPositionRef = useRef<number | null>(null);

  // Update highlighted HTML when value, keywords, or schema changes
  useEffect(() => {
    if (!value) {
      setHighlightedHTML('');
      return;
    }

    let cancelled = false;

    highlightSQL(value, { keywords, schema }).then(html => {
      if (!cancelled) {
        setHighlightedHTML(html);
      }
    });

    return () => {
      cancelled = true;
    };
  }, [value, keywords, schema]);

  // Update cursor position and calculate dropdown position
  useEffect(() => {
    if (!textareaRef.current || !isOpen) return;

    const textarea = textareaRef.current;
    const caret = getCaretCoordinates(textarea, cursorPosition);

    setPosition({
      top: caret.top + caret.height,
      left: caret.left,
    });

    // Update position on scroll
    const handleScroll = () => {
      const newCaret = getCaretCoordinates(textarea, cursorPosition);
      setPosition({
        top: newCaret.top + newCaret.height,
        left: newCaret.left,
      });
    };

    textarea.addEventListener("scroll", handleScroll);
    return () => textarea.removeEventListener("scroll", handleScroll);
  }, [cursorPosition, isOpen]);

  // Update suggestions when value or cursor position changes
  useEffect(() => {
    if (!textareaRef.current) return;

    // Don't show suggestions if nothing is typed
    if (!value.trim()) {
      setIsOpen(false);
      return;
    }

    // Skip autocomplete if cursor hasn't moved since last insertion
    if (lastInsertedPositionRef.current === cursorPosition) {
      return;
    }

    // Clear the insertion tracking once cursor moves
    if (lastInsertedPositionRef.current !== null) {
      lastInsertedPositionRef.current = null;
    }

    const context = detectContext(value, cursorPosition);

    if (context.type === "none") {
      setIsOpen(false);
      return;
    }

    const newSuggestions = generateSuggestions(context, schema, keywords);

    if (newSuggestions.length > 0) {
      setSuggestions(newSuggestions);
      setSelectedIndex(0);
      setIsOpen(true);
    } else {
      setIsOpen(false);
    }
  }, [value, cursorPosition, schema, keywords]);

  // Scroll selected item into view
  useEffect(() => {
    if (!dropdownRef.current || !isOpen) return;

    const dropdown = dropdownRef.current;
    const selectedElement = dropdown.querySelector(
      `[data-index="${selectedIndex}"]`
    ) as HTMLElement;

    if (selectedElement) {
      selectedElement.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex, isOpen]);

  const handleTextareaChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    const newCursorPos = e.target.selectionStart;

    // Trigger keyword loading on first keystroke
    if (!hasTypedRef.current && newValue.length > 0 && onFirstKeystroke) {
      hasTypedRef.current = true;
      onFirstKeystroke();
    }

    onChange(newValue);
    setCursorPosition(newCursorPos);
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (!isOpen) {
      // Ctrl+Space to manually trigger autocomplete
      if (e.key === " " && e.ctrlKey) {
        e.preventDefault();
        const context = detectContext(value, cursorPosition);
        const newSuggestions = generateSuggestions(context, schema, keywords);
        if (newSuggestions.length > 0) {
          setSuggestions(newSuggestions);
          setSelectedIndex(0);
          setIsOpen(true);
        }
      }
      return;
    }

    // Handle keyboard navigation when popover is open
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setSelectedIndex((prev) => (prev + 1) % suggestions.length);
        break;

      case "ArrowUp":
        e.preventDefault();
        setSelectedIndex((prev) => (prev - 1 + suggestions.length) % suggestions.length);
        break;

      case "Enter":
      case "Tab":
        if (suggestions[selectedIndex]) {
          e.preventDefault();
          insertSelectedSuggestion(suggestions[selectedIndex]);
        }
        break;

      case "Escape":
        e.preventDefault();
        setIsOpen(false);
        break;
    }
  };

  const handleClick = () => {
    if (textareaRef.current) {
      setCursorPosition(textareaRef.current.selectionStart);
    }
  };

  const insertSelectedSuggestion = (suggestion: Suggestion) => {
    const { newText, newCursorPos } = insertSuggestion(
      value,
      cursorPosition,
      suggestion.value + " " // Add space after suggestion
    );

    // Track the cursor position after insertion
    lastInsertedPositionRef.current = newCursorPos;

    // Update state with new text and cursor position
    onChange(newText);
    setCursorPosition(newCursorPos);
    setIsOpen(false);

    // Set cursor position in the textarea element
    setTimeout(() => {
      if (textareaRef.current) {
        textareaRef.current.selectionStart = newCursorPos;
        textareaRef.current.selectionEnd = newCursorPos;
        textareaRef.current.focus();
      }
    }, 0);
  };

  // Get category icon
  const getCategoryIcon = (category: string) => {
    switch (category) {
      case "keyword":
        return "🔤";
      case "table":
        return "📊";
      case "column":
        return "📝";
      case "function":
        return "⚡";
      default:
        return "";
    }
  };

  // Sync scroll between textarea and highlight overlay
  const handleScroll = () => {
    if (textareaRef.current && highlightRef.current) {
      highlightRef.current.scrollTop = textareaRef.current.scrollTop;
      highlightRef.current.scrollLeft = textareaRef.current.scrollLeft;
    }
  };

  return (
    <div className="relative w-full h-full">
      {/* Syntax highlighting overlay */}
      <div
        ref={highlightRef}
        className="absolute inset-0 overflow-hidden pointer-events-none whitespace-pre-wrap break-words px-3 py-2 text-base md:text-sm font-mono"
        dangerouslySetInnerHTML={{ __html: highlightedHTML }}
      />

      {/* Textarea with transparent text */}
      <Textarea
        ref={textareaRef}
        value={value}
        onChange={handleTextareaChange}
        onKeyDown={handleKeyDown}
        onClick={handleClick}
        onScroll={handleScroll}
        onBlur={() => {
          // Delay closing to allow click on suggestion
          setTimeout(() => setIsOpen(false), 200);
        }}
        placeholder={placeholder}
        disabled={disabled}
        className={`${className} relative z-10 bg-transparent caret-foreground`}
        style={{
          color: 'transparent',
          WebkitTextFillColor: 'transparent',
        }}
        spellCheck={false}
      />

      {isOpen && suggestions.length > 0 && (
        <div
          ref={dropdownRef}
          className="absolute z-50 w-[350px] max-h-[280px] overflow-y-auto rounded-md border bg-popover shadow-md"
          style={{
            top: `${position.top}px`,
            left: `${position.left}px`,
          }}
          onMouseDown={(e) => e.preventDefault()} // Prevent blur on click
        >
          <div className="p-1">
            {suggestions.map((suggestion, index) => (
              <div
                key={`${suggestion.category}-${suggestion.label}-${index}`}
                data-index={index}
                onClick={() => insertSelectedSuggestion(suggestion)}
                className={`px-2 py-1.5 cursor-pointer text-sm rounded-sm transition-colors ${
                  index === selectedIndex
                    ? "bg-accent text-accent-foreground"
                    : "hover:bg-accent/50"
                }`}
              >
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2 flex-1 min-w-0">
                    <span className="text-xs flex-shrink-0 opacity-60">
                      {getCategoryIcon(suggestion.category)}
                    </span>
                    <span className="font-mono text-sm truncate">
                      {suggestion.label}
                    </span>
                  </div>
                  {suggestion.description && (
                    <span className="text-xs text-muted-foreground flex-shrink-0 font-mono">
                      {suggestion.description}
                    </span>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
