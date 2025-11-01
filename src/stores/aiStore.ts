import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import type {
  AiSession,
  ChatMessage,
  AiMode,
  AiTokenPayload,
  AiTableDataPayload,
  AiChartDataPayload,
  AiStatisticPayload,
  AiCompletePayload,
  AiErrorPayload,
} from "@/types/ai.types";
import { ErrorHandler } from "@/lib/ErrorHandler";

interface IAiStore {
  // State
  session: AiSession | null;
  isGenerating: boolean;
  error: string | null;
  isPanelOpen: boolean;
  currentMode: AiMode;

  // Event listeners
  unlistenFns: UnlistenFn[];

  // Actions
  initializeSession: (connectionId: string) => Promise<void>;
  sendMessage: (content: string) => Promise<void>;
  setPanelOpen: (open: boolean) => void;
  setMode: (mode: AiMode) => void;
  clearSession: () => void;
  cleanupListeners: () => void;

  // Internal helpers
  setupEventListeners: (sessionId: string) => Promise<void>;
  appendTokenToLastMessage: (token: string) => void;
  addTableData: (data: any) => void;
  addChartData: (config: any, data: any) => void;
  addStatisticData: (value: number | string, label: string) => void;
  markComplete: () => void;
  setError: (error: string) => void;
}

export const useAiStore = create<IAiStore>((set, get) => ({
  // Initial state
  session: null,
  isGenerating: false,
  error: null,
  isPanelOpen: true,
  currentMode: 'sql',
  unlistenFns: [],

  initializeSession: async (connectionId: string) => {
    try {
      const sessionId = `session-${Date.now()}`;
      const session: AiSession = {
        id: sessionId,
        connectionId,
        messages: [],
        createdAt: new Date(),
        lastActivity: new Date(),
      };

      set({ session, error: null });

      // Set up event listeners for this session
      await get().setupEventListeners(sessionId);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Failed to initialize AI session";
      set({ error: errorMessage });
      ErrorHandler.handle(error, "Failed to initialize AI session");
    }
  },

  setupEventListeners: async (sessionId: string) => {
    const unlistenFns: UnlistenFn[] = [];

    try {
      // Token streaming
      const unlistenToken = await listen<AiTokenPayload>('ai_token', (event) => {
        if (event.payload.session_id === sessionId) {
          get().appendTokenToLastMessage(event.payload.content);
        }
      });
      unlistenFns.push(unlistenToken);

      // Table data
      const unlistenTable = await listen<AiTableDataPayload>('ai_table_data', (event) => {
        if (event.payload.session_id === sessionId) {
          get().addTableData(event.payload.data);
        }
      });
      unlistenFns.push(unlistenTable);

      // Chart data
      const unlistenChart = await listen<AiChartDataPayload>('ai_chart_data', (event) => {
        if (event.payload.session_id === sessionId) {
          get().addChartData(event.payload.config, event.payload.data);
        }
      });
      unlistenFns.push(unlistenChart);

      // Statistic
      const unlistenStat = await listen<AiStatisticPayload>('ai_statistic', (event) => {
        if (event.payload.session_id === sessionId) {
          get().addStatisticData(event.payload.value, event.payload.label);
        }
      });
      unlistenFns.push(unlistenStat);

      // Complete
      const unlistenComplete = await listen<AiCompletePayload>('ai_complete', (event) => {
        if (event.payload.session_id === sessionId) {
          get().markComplete();
        }
      });
      unlistenFns.push(unlistenComplete);

      // Error
      const unlistenError = await listen<AiErrorPayload>('ai_error', (event) => {
        if (event.payload.session_id === sessionId) {
          get().setError(event.payload.error);
        }
      });
      unlistenFns.push(unlistenError);

      set({ unlistenFns });
    } catch (error) {
      ErrorHandler.handle(error, "Failed to set up event listeners");
    }
  },

  sendMessage: async (content: string) => {
    const { session } = get();
    if (!session) {
      ErrorHandler.warning("No session", "Please connect to a database first");
      return;
    }

    set({ isGenerating: true, error: null });

    try {
      // Add user message immediately
      const userMessage: ChatMessage = {
        id: `msg-${Date.now()}`,
        role: 'user',
        content,
        timestamp: new Date(),
        mode: get().currentMode,
      };

      // Add empty assistant message (will be filled via streaming)
      const assistantMessage: ChatMessage = {
        id: `msg-${Date.now() + 1}`,
        role: 'assistant',
        content: '',
        timestamp: new Date(),
        mode: get().currentMode,
      };

      const updatedMessages = [...session.messages, userMessage, assistantMessage];
      set({
        session: {
          ...session,
          messages: updatedMessages,
          lastActivity: new Date(),
        },
      });

      // Invoke Tauri command (non-blocking, agent runs in background)
      await invoke('stream_ai_chat', {
        sessionId: session.id,
        message: content,
        connectionId: session.connectionId,
      });

      // Note: isGenerating will be set to false by the 'ai_complete' or 'ai_error' event
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Failed to send message";
      set({ error: errorMessage, isGenerating: false });
      ErrorHandler.handle(error, "Failed to send message");
    }
  },

  appendTokenToLastMessage: (token: string) => {
    const { session } = get();
    if (!session || session.messages.length === 0) return;

    const messages = [...session.messages];
    const lastMessage = messages[messages.length - 1];

    if (lastMessage.role === 'assistant') {
      lastMessage.content += token;
      set({
        session: {
          ...session,
          messages,
          lastActivity: new Date(),
        },
      });
    }
  },

  addTableData: (data: any) => {
    const { session } = get();
    if (!session || session.messages.length === 0) return;

    const messages = [...session.messages];
    const lastMessage = messages[messages.length - 1];

    if (lastMessage.role === 'assistant') {
      lastMessage.tableData = data;
      set({
        session: {
          ...session,
          messages,
          lastActivity: new Date(),
        },
      });
    }
  },

  addChartData: (config: any, data: any) => {
    const { session } = get();
    if (!session || session.messages.length === 0) return;

    const messages = [...session.messages];
    const lastMessage = messages[messages.length - 1];

    if (lastMessage.role === 'assistant') {
      lastMessage.chartData = { config, data };
      set({
        session: {
          ...session,
          messages,
          lastActivity: new Date(),
        },
      });
    }
  },

  addStatisticData: (value: number | string, label: string) => {
    const { session } = get();
    if (!session || session.messages.length === 0) return;

    const messages = [...session.messages];
    const lastMessage = messages[messages.length - 1];

    if (lastMessage.role === 'assistant') {
      lastMessage.statisticData = { value, label };
      set({
        session: {
          ...session,
          messages,
          lastActivity: new Date(),
        },
      });
    }
  },

  markComplete: () => {
    set({ isGenerating: false });
  },

  setError: (error: string) => {
    set({ error, isGenerating: false });
    ErrorHandler.handle(error, "AI Agent Error");
  },

  setPanelOpen: (open: boolean) => {
    set({ isPanelOpen: open });
  },

  setMode: (mode: AiMode) => {
    set({ currentMode: mode });
  },

  clearSession: () => {
    get().cleanupListeners();
    set({ session: null, error: null, isGenerating: false });
  },

  cleanupListeners: () => {
    const { unlistenFns } = get();
    unlistenFns.forEach(fn => fn());
    set({ unlistenFns: [] });
  },
}));
