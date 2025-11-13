export interface QueryHistoryEntry {
  id: string;
  query: string;
  connection_id: string;
  executed_at: string;
  execution_time_ms: number;
  success: boolean;
}
