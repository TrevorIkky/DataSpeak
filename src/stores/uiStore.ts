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
}));
