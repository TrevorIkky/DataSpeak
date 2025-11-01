import type { Connection, Schema, ConnectionStatus } from "@/types/database.types";
import type { AppSettings } from "@/types/settings.types";
import type { Tab } from "@/types/query.types";
import type { GeographicCell } from "@/types/geography.types";

export interface ISettingsStore {
  settings: AppSettings | null;
  isLoading: boolean;
  loadSettings: () => Promise<void>;
  saveSettings: (settings: AppSettings) => Promise<void>;
  updateApiKey: (apiKey: string) => void;
  updateModels: (textToSql: string, visualization: string) => void;
}

export interface IConnectionStore {
  connections: Connection[];
  activeConnection: Connection | null;
  status: ConnectionStatus;
  isLoading: boolean;
  loadConnections: () => Promise<void>;
  saveConnection: (connection: Partial<Connection>) => Promise<Connection>;
  updateConnection: (id: string, connection: Partial<Connection>) => Promise<Connection>;
  deleteConnection: (id: string) => Promise<void>;
  setActiveConnection: (connection: Connection | null) => void;
  testConnection: (connection: Partial<Connection>) => Promise<{ success: boolean; message: string }>;
}

export interface ISchemaStore {
  schema: Schema | null;
  isLoading: boolean;
  error: string | null;
  loadSchema: (connectionId: string) => Promise<void>;
  clearSchema: () => void;
}

export interface IQueryStore {
  tabs: Tab[];
  activeTabId: string | null;
  addTab: (query?: string) => void;
  addTableTab: (tableName: string) => void;
  addChatTab: () => void;
  removeTab: (id: string) => void;
  setActiveTab: (id: string) => void;
  updateTabQuery: (id: string, query: string) => void;
  updateTab: (id: string, updates: Partial<Tab>) => void;
  executeQuery: (id: string, connectionId: string) => Promise<void>;
  loadTableData: (id: string, connectionId: string) => Promise<void>;
}

export interface IUIStore {
  sidebarOpen: boolean;
  settingsDialogOpen: boolean;
  connectionDialogOpen: boolean;
  editingConnectionId: string | null;
  erdViewerOpen: boolean;
  mobileConnectionsOpen: boolean;
  mobileSchemaOpen: boolean;
  deleteConnectionDialogOpen: boolean;
  exportDialogOpen: boolean;
  importDialogOpen: boolean;
  isGeneratingVisualization: boolean;
  popoverOpen: boolean;
  selectedGeography: GeographicCell | null;
  isMapFullscreen: boolean;
  toggleSidebar: () => void;
  setSettingsDialogOpen: (open: boolean) => void;
  setConnectionDialogOpen: (open: boolean, editingConnectionId?: string | null) => void;
  setErdViewerOpen: (open: boolean) => void;
  setMobileConnectionsOpen: (open: boolean) => void;
  setMobileSchemaOpen: (open: boolean) => void;
  setDeleteConnectionDialogOpen: (open: boolean) => void;
  setExportDialogOpen: (open: boolean) => void;
  setImportDialogOpen: (open: boolean) => void;
  setIsGeneratingVisualization: (isGenerating: boolean) => void;
  setPopoverOpen: (open: boolean) => void;
  setSelectedGeography: (geography: GeographicCell | null) => void;
  setIsMapFullscreen: (fullscreen: boolean) => void;
}
