import type { QueryResult } from "./query.types";
import type { GeometryData } from "./geography.types";

export type OpenRouterModel = {
  id: string;
  name: string;
  provider: string;
};

export type AIModelConfig = {
  textToSqlModel: string;
  visualizationModel: string;
};

export type VisualizationType = "bar" | "line" | "pie" | "scatter" | "area" | "table" | "radar" | "radial";

export type VisualizationConfig = {
  type: VisualizationType;
  title: string;
  description?: string;
  config: {
    x_axis: string;
    y_axis: string[];
    category?: string;
  };
  insights?: string[];
};

export type AIGenerationStatus = "idle" | "generating" | "success" | "error";

// AI Chat & Session Types
export type AiMode = 'sql' | 'analyst' | 'explain' | 'insights' | 'quality';

export type ChatRole = 'user' | 'assistant' | 'system';

export type MessageAction = {
  id: string;
  label: string;
  icon?: string;
  action: 'run_query' | 'edit_query' | 'explain' | 'visualize' | 'copy';
  data?: any;
};

export type ChatMessage = {
  id: string;
  role: ChatRole;
  content: string;
  timestamp: Date;
  mode?: AiMode;
  metadata?: {
    query?: string;
    affectedRows?: number;
    executionTime?: number;
  };
  actions?: MessageAction[];
  // Enhanced fields for inline rendering
  tableData?: QueryResult;
  chartData?: {
    config: VisualizationConfig;
    data: QueryResult;
  };
  statisticData?: {
    value: number | string;
    label: string;
  };
  mapData?: {
    geometry: GeometryData;
    title?: string;
    description?: string;
  };
};

export type AiSession = {
  id: string;
  connectionId: string;
  messages: ChatMessage[];
  createdAt: Date;
  lastActivity: Date;
};

export type AiGenerationResult = {
  content: string;
  isSafe: boolean;
  explanation?: string;
  suggestedActions?: MessageAction[];
};

export type StreamedResponse = {
  sessionId: string;
  messageId: string;
  complete: boolean;
};

// Tauri Event Payloads
export type AiTokenPayload = {
  session_id: string;
  content: string;
};

export type AiTableDataPayload = {
  session_id: string;
  data: QueryResult;
};

export type AiChartDataPayload = {
  session_id: string;
  config: VisualizationConfig;
  data: QueryResult;
};

export type AiStatisticPayload = {
  session_id: string;
  value: number | string;
  label: string;
};

export type AiMapDataPayload = {
  session_id: string;
  geometry: GeometryData;
  title?: string;
  description?: string;
};

export type AiCompletePayload = {
  session_id: string;
  answer: string;
};

export type AiErrorPayload = {
  session_id: string;
  error: string;
};
