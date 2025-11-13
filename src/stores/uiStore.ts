import { create } from "zustand";
import type { IUIStore } from "@/interfaces/store.interface";

export const useUIStore = create<IUIStore>((set) => ({
  sidebarOpen: true,
  settingsDialogOpen: false,
  connectionDialogOpen: false,
  editingConnectionId: null,
  erdViewerOpen: false,
  mobileConnectionsOpen: false,
  mobileSchemaOpen: false,
  deleteConnectionDialogOpen: false,
  exportDialogOpen: false,
  importDialogOpen: false,
  isGeneratingVisualization: false,
  popoverOpen: false,
  selectedGeography: null,
  isMapFullscreen: false,
  openConnectionId: null,
  executionMode: 'current',
  // AI Query Generation
  aiQueryWindowOpen: false,
  aiQueryWindowPosition: { x: 0, y: 0 },
  aiQueryOriginalQuery: "",
  aiQueryGeneratedSql: "",
  aiQueryThinkingContent: "",
  aiQueryError: null,
  isAiQueryGenerating: false,

  toggleSidebar: () => {
    set((state) => ({ sidebarOpen: !state.sidebarOpen }));
  },

  setSettingsDialogOpen: (open: boolean) => {
    set({ settingsDialogOpen: open });
  },

  setConnectionDialogOpen: (open: boolean, editingConnectionId = null) => {
    set({ connectionDialogOpen: open, editingConnectionId });
  },

  setErdViewerOpen: (open: boolean) => {
    set({ erdViewerOpen: open });
  },

  setMobileConnectionsOpen: (open: boolean) => {
    set({ mobileConnectionsOpen: open });
  },

  setMobileSchemaOpen: (open: boolean) => {
    set({ mobileSchemaOpen: open });
  },

  setDeleteConnectionDialogOpen: (open: boolean) => {
    set({ deleteConnectionDialogOpen: open });
  },

  setExportDialogOpen: (open: boolean) => {
    set({ exportDialogOpen: open });
  },

  setImportDialogOpen: (open: boolean) => {
    set({ importDialogOpen: open });
  },

  setIsGeneratingVisualization: (isGenerating: boolean) => {
    set({ isGeneratingVisualization: isGenerating });
  },

  setPopoverOpen: (open: boolean) => {
    set({ popoverOpen: open });
  },

  setSelectedGeography: (geography) => {
    set({ selectedGeography: geography });
  },

  setIsMapFullscreen: (fullscreen: boolean) => {
    set({ isMapFullscreen: fullscreen });
  },

  setOpenConnectionId: (connectionId: string | null) => {
    set({ openConnectionId: connectionId });
  },

  setExecutionMode: (mode: 'current' | 'all') => {
    set({ executionMode: mode });
  },

  openAiQueryWindow: (position, originalQuery) => {
    set({
      aiQueryWindowOpen: true,
      aiQueryWindowPosition: position,
      aiQueryOriginalQuery: originalQuery,
      aiQueryGeneratedSql: "",
      aiQueryThinkingContent: "",
      aiQueryError: null,
      isAiQueryGenerating: false,
    });
  },

  closeAiQueryWindow: () => {
    set({
      aiQueryWindowOpen: false,
      aiQueryOriginalQuery: "",
      aiQueryGeneratedSql: "",
      aiQueryThinkingContent: "",
      aiQueryError: null,
      isAiQueryGenerating: false,
    });
  },

  startAiQueryGeneration: () => {
    set({ isAiQueryGenerating: true, aiQueryGeneratedSql: "", aiQueryThinkingContent: "", aiQueryError: null });
  },

  updateAiQueryThinkingContent: (thinkingContent) => {
    set({ aiQueryThinkingContent: thinkingContent });
  },

  updateAiQuerySql: (generatedSql) => {
    set({ aiQueryGeneratedSql: generatedSql });
  },

  completeAiQueryGeneration: (generatedSql, thinkingContent) => {
    set({
      isAiQueryGenerating: false,
      aiQueryGeneratedSql: generatedSql,
      aiQueryThinkingContent: thinkingContent,
    });
  },

  setAiQueryError: (error) => {
    set({
      isAiQueryGenerating: false,
      aiQueryError: error,
    });
  },
}));
