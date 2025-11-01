import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { Connection, ConnectionStatus } from "@/types/database.types";
import type { IConnectionStore } from "@/interfaces/store.interface";
import { ErrorHandler } from "@/lib/ErrorHandler";

export const useConnectionStore = create<IConnectionStore>((set, get) => ({
  connections: [],
  activeConnection: null,
  status: "disconnected",
  isLoading: false,

  loadConnections: async () => {
    set({ isLoading: true });
    try {
      const connections = await invoke<Connection[]>("get_connections");
      set({ connections, isLoading: false });
    } catch (error) {
      ErrorHandler.handle(error, "Failed to load connections");
      set({ isLoading: false });
    }
  },

  saveConnection: async (connection: Partial<Connection>) => {
    set({ isLoading: true });
    try {
      const saved = await invoke<Connection>("save_connection", { connection });
      const connections = [...get().connections, saved];
      set({ connections, isLoading: false });
      ErrorHandler.success("Connection saved successfully");
      return saved;
    } catch (error) {
      ErrorHandler.handle(error, "Failed to save connection");
      set({ isLoading: false });
      throw error;
    }
  },

  updateConnection: async (id: string, connection: Partial<Connection>) => {
    set({ isLoading: true });
    try {
      const updated = await invoke<Connection>("update_connection", { id, connection });
      const connections = get().connections.map((c) => (c.id === id ? updated : c));

      // Update active connection if it's the one being edited
      const { activeConnection } = get();
      const newActiveConnection = activeConnection?.id === id ? updated : activeConnection;

      set({ connections, activeConnection: newActiveConnection, isLoading: false });
      ErrorHandler.success("Connection updated successfully");
      return updated;
    } catch (error) {
      ErrorHandler.handle(error, "Failed to update connection");
      set({ isLoading: false });
      throw error;
    }
  },

  deleteConnection: async (id: string) => {
    set({ isLoading: true });
    try {
      await invoke("delete_connection", { id });
      const connections = get().connections.filter((c) => c.id !== id);
      const { activeConnection } = get();

      if (activeConnection?.id === id) {
        set({ connections, activeConnection: null, isLoading: false });
      } else {
        set({ connections, isLoading: false });
      }

      ErrorHandler.success("Connection deleted successfully");
    } catch (error) {
      ErrorHandler.handle(error, "Failed to delete connection");
      set({ isLoading: false });
      throw error;
    }
  },

  setActiveConnection: (connection: Connection | null) => {
    set({
      activeConnection: connection,
      status: connection ? "connected" : "disconnected",
    });
  },

  testConnection: async (connection: Partial<Connection>) => {
    try {
      const result = await invoke<{ success: boolean; message: string }>(
        "test_connection",
        { connection }
      );

      if (result.success) {
        ErrorHandler.success("Connection successful", result.message);
      } else {
        ErrorHandler.warning("Connection failed", result.message);
      }

      return result;
    } catch (error) {
      ErrorHandler.handle(error, "Failed to test connection");
      return { success: false, message: "Connection test failed" };
    }
  },
}));
