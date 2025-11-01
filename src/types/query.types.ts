export type QueryResult = {
  columns: string[];
  rows: Record<string, any>[];
  row_count: number;
  execution_time_ms: number;
};

export type PaginationState = {
  pageIndex: number;
  pageSize: number;
};

export type SortingState = {
  id: string;
  desc: boolean;
}[];

export type TabType = 'query' | 'table' | 'visualization' | 'chat' | 'erd';

export type BaseTab = {
  id: string;
  title: string;
  isLoading: boolean;
  isActive: boolean;
  error?: string;
};

export type QueryTab = BaseTab & {
  type: 'query';
  query: string;
  result?: QueryResult;
  chartConfig?: import('./ai.types').VisualizationConfig | null;
  showVisualization?: boolean;
};

export type TableTab = BaseTab & {
  type: 'table';
  tableName: string;
  result?: QueryResult;
  pagination: {
    pageIndex: number;
    pageSize: number;
  };
  viewMode: 'data' | 'properties' | 'erd';
};

export type VisualizationTab = BaseTab & {
  type: 'visualization';
  queryResult: QueryResult;
  chartConfig: import('./ai.types').VisualizationConfig | null;
  showGrid: boolean; // for split view toggle
};

export type ChatTab = BaseTab & {
  type: 'chat';
};

export type Tab = QueryTab | TableTab | VisualizationTab | ChatTab;
