import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { Tab, QueryTab, TableTab, ChatTab, QueryResult } from "@/types/query.types";
import type { IQueryStore } from "@/interfaces/store.interface";
import { ErrorHandler } from "@/lib/ErrorHandler";

export const useQueryStore = create<IQueryStore>((set, get) => ({
  tabs: [],
  activeTabId: null,

  addTab: (query = "") => {
    const id = `tab-${Date.now()}`;
    const newTab: QueryTab = {
      id,
      type: 'query',
      title: `Query ${get().tabs.length + 1}`,
      query,
      isLoading: false,
      isActive: true,
    };

    // Set all other tabs to inactive
    const tabs = get().tabs.map((tab) => ({ ...tab, isActive: false }));
    tabs.push(newTab);

    set({ tabs, activeTabId: id });
  },

  addTableTab: (tableName: string) => {
    // Check if table tab already exists
    const existingTab = get().tabs.find(
      (tab) => tab.type === 'table' && (tab as TableTab).tableName === tableName
    );

    if (existingTab) {
      // Switch to existing tab
      get().setActiveTab(existingTab.id);
      return;
    }

    const id = `table-${tableName}-${Date.now()}`;
    const newTab: TableTab = {
      id,
      type: 'table',
      title: tableName,
      tableName,
      isLoading: false,
      isActive: true,
      pagination: {
        pageIndex: 0,
        pageSize: 50,
      },
      viewMode: 'data',
    };

    // Set all other tabs to inactive
    const tabs = get().tabs.map((tab) => ({ ...tab, isActive: false }));
    tabs.push(newTab);

    set({ tabs, activeTabId: id });
  },

  addChatTab: () => {
    // Check if chat tab already exists
    const existingTab = get().tabs.find(
      (tab) => tab.type === 'chat'
    );

    if (existingTab) {
      // Switch to existing tab
      get().setActiveTab(existingTab.id);
      return;
    }

    const id = `chat-${Date.now()}`;
    const newTab: ChatTab = {
      id,
      type: 'chat',
      title: 'AI Assistant',
      isLoading: false,
      isActive: true,
    };

    // Set all other tabs to inactive
    const tabs = get().tabs.map((tab) => ({ ...tab, isActive: false }));
    tabs.push(newTab);

    set({ tabs, activeTabId: id });
  },

  removeTab: (id: string) => {
    const tabs = get().tabs.filter((tab) => tab.id !== id);
    const { activeTabId } = get();

    if (activeTabId === id) {
      const newActiveTab = tabs[tabs.length - 1];
      if (newActiveTab) {
        set({
          tabs: tabs.map((tab) => ({
            ...tab,
            isActive: tab.id === newActiveTab.id,
          })),
          activeTabId: newActiveTab.id,
        });
      } else {
        set({ tabs: [], activeTabId: null });
      }
    } else {
      set({ tabs });
    }
  },

  setActiveTab: (id: string) => {
    const tabs = get().tabs.map((tab) => ({
      ...tab,
      isActive: tab.id === id,
    }));
    set({ tabs, activeTabId: id });
  },

  updateTabQuery: (id: string, query: string) => {
    const tabs = get().tabs.map((tab) => {
      if (tab.id === id && tab.type === 'query') {
        return { ...tab, query } as QueryTab;
      }
      return tab;
    });
    set({ tabs });
  },

  updateTab: (id: string, updates: Partial<Tab>) => {
    const tabs = get().tabs.map((tab) => {
      if (tab.id === id) {
        return { ...tab, ...updates };
      }
      return tab;
    });
    set({ tabs });
  },

  executeQuery: async (id: string, connectionId: string) => {
    const tab = get().tabs.find((t) => t.id === id);
    if (!tab || tab.type !== 'query') return;

    // Set tab to loading
    set({
      tabs: get().tabs.map((t) =>
        t.id === id
          ? { ...t, isLoading: true, error: undefined, result: undefined }
          : t
      ),
    });

    try {
      const result = await invoke<QueryResult>("run_query", {
        connectionId,
        query: tab.query,
        limit: 50,
        offset: 0,
      });

      set({
        tabs: get().tabs.map((t) =>
          t.id === id ? { ...t, isLoading: false, result } : t
        ),
      });

      ErrorHandler.success(
        `Query executed successfully`,
        `${result.row_count} rows returned in ${result.execution_time_ms}ms`
      );
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : "Query execution failed";

      set({
        tabs: get().tabs.map((t) =>
          t.id === id ? { ...t, isLoading: false, error: errorMessage } : t
        ),
      });

      ErrorHandler.handle(error, "Query execution failed");
    }
  },

  loadTableData: async (id: string, connectionId: string) => {
    const tab = get().tabs.find((t) => t.id === id);
    if (!tab || tab.type !== 'table') return;

    const tableTab = tab as TableTab;

    // Set tab to loading
    set({
      tabs: get().tabs.map((t) =>
        t.id === id
          ? { ...t, isLoading: true, error: undefined }
          : t
      ),
    });

    try {
      const { pageIndex, pageSize } = tableTab.pagination;
      const offset = pageIndex * pageSize;

      const result = await invoke<QueryResult>("run_query", {
        connectionId,
        query: `SELECT * FROM ${tableTab.tableName}`,
        limit: pageSize,
        offset,
      });

      set({
        tabs: get().tabs.map((t) =>
          t.id === id ? { ...t, isLoading: false, result } : t
        ),
      });
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : "Failed to load table data";

      set({
        tabs: get().tabs.map((t) =>
          t.id === id ? { ...t, isLoading: false, error: errorMessage } : t
        ),
      });

      ErrorHandler.handle(error, "Failed to load table data");
    }
  },
}));
