export type AppSettings = {
  openrouter_api_key: string;
  text_to_sql_model: string;
  visualization_model: string;
  conversation_history_limit: number;
};

export type Theme = "light" | "dark" | "system";
