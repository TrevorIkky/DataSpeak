pub mod stronghold;

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

pub use stronghold::StrongholdStorage;

pub struct StorageManager {
    settings: Mutex<Option<AppSettings>>,
    app_data_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub openrouter_api_key: String,
    pub text_to_sql_model: String,
    pub visualization_model: String,
    #[serde(default = "default_conversation_history_limit")]
    pub conversation_history_limit: usize,
}

fn default_conversation_history_limit() -> usize {
    10
}

impl StorageManager {
    pub fn new(app_handle: &tauri::AppHandle) -> AppResult<Self> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| AppError::StorageError(format!("Failed to get app data dir: {}", e)))?;

        // Ensure the directory exists
        fs::create_dir_all(&app_data_dir)
            .map_err(|e| AppError::StorageError(format!("Failed to create app data dir: {}", e)))?;

        Ok(Self {
            settings: Mutex::new(None),
            app_data_dir,
        })
    }

    pub fn save_settings(&self, settings: AppSettings) -> AppResult<()> {
        let mut guard = self.settings.lock().map_err(|e| {
            AppError::StorageError(format!("Failed to lock settings: {}", e))
        })?;
        *guard = Some(settings.clone());

        // Persist to file
        let settings_path = self.app_data_dir.join("settings.json");
        let json = serde_json::to_string_pretty(&settings)
            .map_err(|e| AppError::StorageError(format!("Failed to serialize settings: {}", e)))?;
        fs::write(settings_path, json)
            .map_err(|e| AppError::StorageError(format!("Failed to write settings file: {}", e)))?;

        Ok(())
    }

    pub fn get_settings(&self) -> AppResult<Option<AppSettings>> {
        let guard = self.settings.lock().map_err(|e| {
            AppError::StorageError(format!("Failed to lock settings: {}", e))
        })?;

        if guard.is_some() {
            return Ok(guard.clone());
        }

        // Try loading from file
        drop(guard);
        self.load_settings()
    }

    pub fn load_settings(&self) -> AppResult<Option<AppSettings>> {
        let settings_path = self.app_data_dir.join("settings.json");

        if !settings_path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(settings_path)
            .map_err(|e| AppError::StorageError(format!("Failed to read settings file: {}", e)))?;
        let settings: AppSettings = serde_json::from_str(&json)
            .map_err(|e| AppError::StorageError(format!("Failed to parse settings: {}", e)))?;

        let mut guard = self.settings.lock().map_err(|e| {
            AppError::StorageError(format!("Failed to lock settings: {}", e))
        })?;
        *guard = Some(settings.clone());

        Ok(Some(settings))
    }
}
