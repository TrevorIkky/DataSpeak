import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "@/types/settings.types";
import type { ISettingsStore } from "@/interfaces/store.interface";
import { ErrorHandler } from "@/lib/ErrorHandler";

export const useSettingsStore = create<ISettingsStore>((set, get) => ({
  settings: null,
  isLoading: false,

  loadSettings: async () => {
    set({ isLoading: true });
    try {
      const settings = await invoke<AppSettings | null>("get_settings");
      set({ settings, isLoading: false });
    } catch (error) {
      ErrorHandler.handle(error, "Failed to load settings");
      set({ isLoading: false });
    }
  },

  saveSettings: async (settings: AppSettings) => {
    set({ isLoading: true });
    try {
      await invoke("save_settings", { settings });
      set({ settings, isLoading: false });
      ErrorHandler.success("Settings saved successfully");
    } catch (error) {
      ErrorHandler.handle(error, "Failed to save settings");
      set({ isLoading: false });
      throw error;
    }
  },

  updateApiKey: (apiKey: string) => {
    const { settings } = get();
    if (settings) {
      set({
        settings: {
          ...settings,
          openrouter_api_key: apiKey,
        },
      });
    }
  },

  updateModels: (textToSql: string, visualization: string) => {
    const { settings } = get();
    if (settings) {
      set({
        settings: {
          ...settings,
          text_to_sql_model: textToSql,
          visualization_model: visualization,
        },
      });
    }
  },
}));
