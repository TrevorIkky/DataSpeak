import { create } from "zustand";

export type EditorInteractionMode = 'typing' | 'pasting' | 'programmatic' | 'idle';

export interface PastedRange {
  start: number;
  end: number;
}

interface EditorStore {
  // State
  interactionMode: EditorInteractionMode;
  pastedRange: PastedRange | null;
  showLineNumbers: boolean;
  lineCount: number;

  // Actions
  setInteractionMode: (mode: EditorInteractionMode) => void;
  setPastedRange: (range: PastedRange | null) => void;
  setShowLineNumbers: (show: boolean) => void;
  setLineCount: (count: number) => void;
  resetToIdle: () => void;
  handlePaste: (start: number, end: number) => void;
  handleTyping: () => void;
  handleProgrammaticChange: () => void;
}

export const useEditorStore = create<EditorStore>((set) => ({
  // Initial state
  interactionMode: 'idle',
  pastedRange: null,
  showLineNumbers: true,
  lineCount: 1,

  // Actions
  setInteractionMode: (mode) => set({ interactionMode: mode }),

  setPastedRange: (range) => set({ pastedRange: range }),

  setShowLineNumbers: (show) => set({ showLineNumbers: show }),

  setLineCount: (count) => set({ lineCount: count }),

  resetToIdle: () => set({ interactionMode: 'idle', pastedRange: null }),

  handlePaste: (start, end) => set({
    interactionMode: 'pasting',
    pastedRange: { start, end },
  }),

  handleTyping: () => set((state) => {
    // Only transition from idle or after paste/programmatic
    if (state.interactionMode !== 'typing') {
      return { interactionMode: 'typing', pastedRange: null };
    }
    return {};
  }),

  handleProgrammaticChange: () => set({
    interactionMode: 'programmatic',
    pastedRange: null,
  }),
}));
