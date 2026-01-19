import { useState, useRef, useEffect, KeyboardEvent, forwardRef, useImperativeHandle } from "react";
import { Textarea } from "@/components/ui/textarea";
import getCaretCoordinates from "textarea-caret";
import {
  detectContext,
  generateSuggestions,
  insertSuggestion,
  type Suggestion,
} from "@/lib/sqlAutocomplete";
import { highlightSQL } from "@/lib/sqlSyntaxHighlight";
import { useEditorStore } from "@/stores/editorStore";
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
  onKeyDown?: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  onSelect?: (e: React.SyntheticEvent<HTMLTextAreaElement>) => void;
  onClick?: (e: React.MouseEvent<HTMLTextAreaElement>) => void;
}

export const SqlAutocompleteTextarea = forwardRef<HTMLTextAreaElement, SqlAutocompleteTextareaProps>(({
  value,
  onChange,
  placeholder,
  disabled,
  className,
  schema,
  keywords,
  onFirstKeystroke,
  onKeyDown: externalOnKeyDown,
  onSelect: externalOnSelect,
  onClick: externalOnClick,
}, ref) => {
  // Zustand store
  const {
    interactionMode,
    pastedRange,
    showLineNumbers,
    lineCount,
    handlePaste,
    handleTyping,
    handleProgrammaticChange,
    setPastedRange,
    setLineCount
  } = useEditorStore();

  // Local state
  const [isOpen, setIsOpen] = useState(false);
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [cursorPosition, setCursorPosition] = useState(0);
  const [position, setPosition] = useState({ top: 0, left: 0 });
  const [highlightedHTML, setHighlightedHTML] = useState('');

  // Refs
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const highlightRef = useRef<HTMLDivElement>(null);
  const pasteHighlightRef = useRef<HTMLDivElement>(null);
  const lineNumbersRef = useRef<HTMLDivElement>(null);
  const hasTypedRef = useRef(false);
  const lastInsertedPositionRef = useRef<number | null>(null);
  const lastValueLengthRef = useRef(0);

  // Expose the internal ref to the parent component
  useImperativeHandle(ref, () => textareaRef.current as HTMLTextAreaElement);

  // Update highlighted HTML when value, keywords, or schema changes
  useEffect(() => {
    if (!value) {
      setHighlightedHTML('');
      setLineCount(1);
      return;
    }

    // Calculate line count
    const lines = value.split('\n').length;
    setLineCount(lines);

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

  // Clear paste highlight after 2 seconds
  useEffect(() => {
    if (!pastedRange) return;

    const timeout = setTimeout(() => {
      setPastedRange(null);
    }, 2000);

    return () => clearTimeout(timeout);
  }, [pastedRange]);

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

  // Detect large changes (paste or programmatic updates)
  useEffect(() => {
    if (!value) {
      lastValueLengthRef.current = 0;
      return;
    }

    const lengthDiff = Math.abs(value.length - lastValueLengthRef.current);

    // Large change detected - likely paste or programmatic update
    if (lengthDiff > 5 && interactionMode !== 'pasting') {
      handleProgrammaticChange();
    }

    lastValueLengthRef.current = value.length;
  }, [value, interactionMode, handleProgrammaticChange]);

  // Reset interaction mode to idle after paste or programmatic change
  useEffect(() => {
    if (interactionMode === 'pasting' || interactionMode === 'programmatic') {
      // Wait for next user keypress to resume typing mode
      const handleKeyPress = () => {
        handleTyping();
      };

      const textarea = textareaRef.current;
      textarea?.addEventListener('keypress', handleKeyPress, { once: true });

      return () => {
        textarea?.removeEventListener('keypress', handleKeyPress);
      };
    }
  }, [interactionMode, handleTyping]);

  // Update suggestions when value or cursor position changes
  useEffect(() => {
    if (!textareaRef.current) return;

    // Don't show suggestions if nothing is typed
    if (!value.trim()) {
      setIsOpen(false);
      return;
    }

    // Only show autocomplete when user is actively typing
    if (interactionMode !== 'typing' && interactionMode !== 'idle') {
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
  }, [value, cursorPosition, schema, keywords, interactionMode]);

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

    // If mode is idle and user is changing text, they're typing
    if (interactionMode === 'idle') {
      const lengthDiff = Math.abs(newValue.length - value.length);
      // Small changes are typing, large changes will be detected by the useEffect
      if (lengthDiff <= 5) {
        handleTyping();
      }
    }

    onChange(newValue);
    setCursorPosition(newCursorPos);
  };

  const onPaste = (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const pastedText = e.clipboardData.getData('text');
    const textarea = e.currentTarget;
    const start = textarea.selectionStart;
    const end = start + pastedText.length;

    // Use store action to handle paste
    // Wait for next animation frame for paste to complete
    requestAnimationFrame(() => {
      handlePaste(start, end);
    });
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
      // Call external handler if provided
      externalOnKeyDown?.(e);
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

  const handleClick = (e: React.MouseEvent<HTMLTextAreaElement>) => {
    if (textareaRef.current) {
      setCursorPosition(textareaRef.current.selectionStart);
    }
    // Call external handler if provided
    externalOnClick?.(e);
  };

  const handleSelect = (e: React.SyntheticEvent<HTMLTextAreaElement>) => {
    if (textareaRef.current) {
      setCursorPosition(textareaRef.current.selectionStart);
    }
    // Call external handler if provided
    externalOnSelect?.(e);
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

  // Render paste highlight overlay
  const renderPasteHighlight = () => {
    if (!pastedRange || !textareaRef.current) return null;

    const beforeText = value.substring(0, pastedRange.start);
    const pastedText = value.substring(pastedRange.start, pastedRange.end);

    // Create invisible content before the pasted text to position it correctly
    return (
      <div
        ref={pasteHighlightRef}
        className="absolute inset-0 overflow-hidden pointer-events-none whitespace-pre-wrap break-words px-2 sm:px-3 py-2 text-xs sm:text-sm font-mono"
        style={{ zIndex: 5 }}
      >
        <span style={{ opacity: 0 }}>{beforeText}</span>
        <span className="sql-paste-flash inline">{pastedText}</span>
      </div>
    );
  };

  // Sync scroll between textarea and highlight overlays
  const handleScroll = () => {
    if (textareaRef.current) {
      if (highlightRef.current) {
        highlightRef.current.scrollTop = textareaRef.current.scrollTop;
        highlightRef.current.scrollLeft = textareaRef.current.scrollLeft;
      }
      if (pasteHighlightRef.current) {
        pasteHighlightRef.current.scrollTop = textareaRef.current.scrollTop;
        pasteHighlightRef.current.scrollLeft = textareaRef.current.scrollLeft;
      }
      if (lineNumbersRef.current) {
        lineNumbersRef.current.scrollTop = textareaRef.current.scrollTop;
      }
    }
  };

  return (
    <div className="relative w-full h-full flex">
      {/* Line numbers */}
      {showLineNumbers && (
        <div
          ref={lineNumbersRef}
          className="flex-shrink-0 overflow-hidden select-none pointer-events-none py-2 pr-2 pl-1 text-xs sm:text-sm font-mono text-muted-foreground/50 border-r border-border/40"
          style={{
            width: `${Math.max(String(lineCount).length * 8 + 12, 32)}px`,
            minWidth: '32px'
          }}
        >
          {Array.from({ length: lineCount }, (_, i) => (
            <div key={i + 1} className="leading-[1.5] text-center">
              {i + 1}
            </div>
          ))}
        </div>
      )}

      {/* Editor area */}
      <div className="relative flex-1 h-full">
        {/* Syntax highlighting overlay */}
        <div
          ref={highlightRef}
          className="absolute inset-0 overflow-hidden pointer-events-none whitespace-pre-wrap break-words px-2 sm:px-3 py-2 text-xs sm:text-sm font-mono"
          dangerouslySetInnerHTML={{ __html: highlightedHTML }}
        />

        {/* Paste highlight overlay */}
        {renderPasteHighlight()}

        {/* Textarea with transparent text */}
        <Textarea
          ref={textareaRef}
          value={value}
          onChange={handleTextareaChange}
          onKeyDown={handleKeyDown}
          onClick={handleClick}
          onSelect={handleSelect}
          onScroll={handleScroll}
          onPaste={onPaste}
          onBlur={() => {
            // Delay closing to allow click on suggestion
            setTimeout(() => setIsOpen(false), 200);
          }}
          placeholder={placeholder}
          disabled={disabled}
          className={`${className} relative z-10 bg-transparent caret-foreground rounded-none border-0 focus-visible:ring-0 focus-visible:ring-offset-0 outline-none`}
          style={{
            color: 'transparent',
            WebkitTextFillColor: 'transparent',
          }}
          spellCheck={false}
        />
      </div>

      {isOpen && suggestions.length > 0 && (
        <div
          ref={dropdownRef}
          className="absolute z-50 w-[280px] sm:w-[350px] max-h-[240px] sm:max-h-[280px] overflow-y-auto rounded-md border bg-popover shadow-md"
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
                className={`px-2 py-1.5 cursor-pointer text-xs sm:text-sm rounded-sm transition-colors ${
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
                    <span className="font-mono text-xs sm:text-sm truncate">
                      {suggestion.label}
                    </span>
                  </div>
                  {suggestion.description && (
                    <span className="text-xs text-muted-foreground flex-shrink-0 font-mono hidden sm:inline">
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
});

SqlAutocompleteTextarea.displayName = 'SqlAutocompleteTextarea';
