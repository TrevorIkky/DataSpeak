import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import type {
  AiSession,
  ChatMessage,
  ChatRole,
  AiMode,
  AiTokenPayload,
  AiThinkingPayload,
  AiTableDataPayload,
  AiChartDataPayload,
  AiPlotlyChartPayload,
  AiStatisticPayload,
  AiCompletePayload,
  AiErrorPayload,
  ConversationMetadata,
} from "@/types/ai.types";
import { ErrorHandler } from "@/lib/ErrorHandler";

// Backend message type for loading conversation history
interface BackendMessage {
  role: string;
  content: string;
  timestamp: number;
}

interface IAiStore {
  // State
  session: AiSession | null;
  isGenerating: boolean;
  error: string | null;
  isPanelOpen: boolean;
  currentMode: AiMode;

  // Conversation management state
  conversations: ConversationMetadata[];
  isLoadingConversations: boolean;
  sidebarOpen: boolean;
  deleteConfirmationId: string | null;

  // Event listeners
  unlistenFns: UnlistenFn[];

  // Actions
  initializeSession: (connectionId: string) => Promise<void>;
  sendMessage: (content: string) => Promise<void>;
  setPanelOpen: (open: boolean) => void;
  setMode: (mode: AiMode) => void;
  clearSession: () => void;
  cleanupListeners: () => void;

  // Conversation management actions
  loadConversations: (connectionId: string) => Promise<void>;
  switchConversation: (sessionId: string) => Promise<void>;
  startNewConversation: () => Promise<void>;
  deleteConversation: (sessionId: string) => Promise<void>;
  setSidebarOpen: (open: boolean) => void;
  setDeleteConfirmationId: (id: string | null) => void;

  // Internal helpers
  setupEventListeners: (sessionId: string) => Promise<void>;
  appendTokenToLastMessage: (token: string) => void;
  appendThinkingToLastMessage: (token: string) => void;
  addTableData: (data: any) => void;
  addChartData: (config: any, data: any) => void;
  addPlotlyChart: (plotlyData: any[], plotlyLayout: any, title: string, chartType: string) => void;
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

  // Conversation management state
  conversations: [],
  isLoadingConversations: false,
  sidebarOpen: false, // Collapsed by default
  deleteConfirmationId: null,

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
      // Token streaming (final answer)
      const unlistenToken = await listen<AiTokenPayload>('ai_token', (event) => {
        if (event.payload.session_id === sessionId) {
          get().appendTokenToLastMessage(event.payload.content);
        }
      });
      unlistenFns.push(unlistenToken);

      // Thinking tokens (pipeline status)
      const unlistenThinking = await listen<AiThinkingPayload>('ai_thinking', (event) => {
        if (event.payload.session_id === sessionId) {
          get().appendThinkingToLastMessage(event.payload.content);
        }
      });
      unlistenFns.push(unlistenThinking);

      // Table data
      const unlistenTable = await listen<AiTableDataPayload>('ai_table_data', (event) => {
        if (event.payload.session_id === sessionId) {
          get().addTableData(event.payload.data);
        }
      });
      unlistenFns.push(unlistenTable);

      // Chart data (legacy)
      const unlistenChart = await listen<AiChartDataPayload>('ai_chart_data', (event) => {
        if (event.payload.session_id === sessionId) {
          get().addChartData(event.payload.config, event.payload.data);
        }
      });
      unlistenFns.push(unlistenChart);

      // Plotly chart (JSON data visualization)
      const unlistenPlotly = await listen<AiPlotlyChartPayload>('ai_plotly_chart', (event) => {
        if (event.payload.session_id === sessionId) {
          get().addPlotlyChart(
            event.payload.plotly_data,
            event.payload.plotly_layout,
            event.payload.title,
            event.payload.chart_type
          );
        }
      });
      unlistenFns.push(unlistenPlotly);

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

  appendThinkingToLastMessage: (token: string) => {
    const { session } = get();
    if (!session || session.messages.length === 0) return;

    const messages = [...session.messages];
    const lastMessage = messages[messages.length - 1];

    if (lastMessage.role === 'assistant') {
      lastMessage.thinking = (lastMessage.thinking || '') + token;
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

  addPlotlyChart: (plotlyData: any[], plotlyLayout: any, title: string, chartType: string) => {
    const { session } = get();
    if (!session || session.messages.length === 0) return;

    const messages = [...session.messages];
    const lastMessage = messages[messages.length - 1];

    if (lastMessage.role === 'assistant') {
      lastMessage.plotlyChart = { plotlyData, plotlyLayout, title, chartType };
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
    // Refresh conversation list after completion
    const { session } = get();
    if (session?.connectionId) {
      get().loadConversations(session.connectionId);
    }
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

  // Conversation management actions
  loadConversations: async (connectionId: string) => {
    set({ isLoadingConversations: true });
    try {
      const conversations = await invoke<ConversationMetadata[]>('list_conversations', {
        connectionId,
      });
      set({ conversations, isLoadingConversations: false });
    } catch (error) {
      ErrorHandler.handle(error, "Failed to load conversations");
      set({ isLoadingConversations: false });
    }
  },

  switchConversation: async (sessionId: string) => {
    const { session, conversations } = get();

    // Cleanup current session listeners
    get().cleanupListeners();

    try {
      // Load messages from backend
      const messages = await invoke<BackendMessage[]>('get_conversation_history', {
        sessionId,
      });

      // Find metadata for this conversation
      const metadata = conversations.find(c => c.session_id === sessionId);

      // Transform backend messages to ChatMessage format
      const chatMessages: ChatMessage[] = messages
        .filter(m => m.role === 'user' || m.role === 'assistant')
        .map((m, index) => ({
          id: `msg-${m.timestamp}-${index}`,
          role: m.role.toLowerCase() as ChatRole,
          content: m.content,
          timestamp: new Date(m.timestamp * 1000),
        }));

      const newSession: AiSession = {
        id: sessionId,
        connectionId: metadata?.connection_id || session?.connectionId || '',
        messages: chatMessages,
        createdAt: new Date((metadata?.created_at || Date.now() / 1000) * 1000),
        lastActivity: new Date((metadata?.updated_at || Date.now() / 1000) * 1000),
      };

      set({ session: newSession, error: null });

      // Setup listeners for new session
      await get().setupEventListeners(sessionId);
    } catch (error) {
      ErrorHandler.handle(error, "Failed to load conversation");
    }
  },

  startNewConversation: async () => {
    const { session } = get();
    const connectionId = session?.connectionId;

    if (!connectionId) return;

    // Cleanup current listeners
    get().cleanupListeners();

    // Create fresh session
    const sessionId = `session-${Date.now()}`;
    const newSession: AiSession = {
      id: sessionId,
      connectionId,
      messages: [],
      createdAt: new Date(),
      lastActivity: new Date(),
    };

    set({ session: newSession, error: null });

    // Setup listeners for new session
    await get().setupEventListeners(sessionId);
  },

  deleteConversation: async (sessionId: string) => {
    try {
      await invoke('clear_conversation', { sessionId });

      // Remove from local state
      const { conversations, session } = get();
      const updatedConversations = conversations.filter(c => c.session_id !== sessionId);
      set({ conversations: updatedConversations, deleteConfirmationId: null });

      // If deleted current session, start new one
      if (session?.id === sessionId) {
        await get().startNewConversation();
      }
    } catch (error) {
      ErrorHandler.handle(error, "Failed to delete conversation");
    }
  },

  setSidebarOpen: (open: boolean) => {
    set({ sidebarOpen: open });
  },

  setDeleteConfirmationId: (id: string | null) => {
    set({ deleteConfirmationId: id });
  },
}));
