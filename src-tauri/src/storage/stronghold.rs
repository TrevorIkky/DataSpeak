use crate::db::connection::Connection;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConnectionIndex {
    pub connections: Vec<ConnectionMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConnectionMetadata {
    pub id: String,
    pub name: String,
}

pub struct StrongholdStorage {
    app_data_dir: PathBuf,
}

impl StrongholdStorage {
    pub fn new(app_handle: &AppHandle) -> AppResult<Self> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| AppError::StorageError(format!("Failed to get app data dir: {}", e)))?;

        // Ensure the directory exists
        fs::create_dir_all(&app_data_dir)
            .map_err(|e| AppError::StorageError(format!("Failed to create app data dir: {}", e)))?;

        Ok(Self { app_data_dir })
    }

    fn load_connection_index(&self) -> AppResult<ConnectionIndex> {
        let index_path = self.app_data_dir.join("connections_index.json");

        if !index_path.exists() {
            return Ok(ConnectionIndex {
                connections: Vec::new(),
            });
        }

        let json = fs::read_to_string(index_path)
            .map_err(|e| AppError::StorageError(format!("Failed to read connections index: {}", e)))?;
        let index: ConnectionIndex = serde_json::from_str(&json)
            .map_err(|e| AppError::StorageError(format!("Failed to parse connections index: {}", e)))?;

        Ok(index)
    }

    fn save_connection_index(&self, index: &ConnectionIndex) -> AppResult<()> {
        let index_path = self.app_data_dir.join("connections_index.json");
        let json = serde_json::to_string_pretty(index)
            .map_err(|e| AppError::StorageError(format!("Failed to serialize connections index: {}", e)))?;
        fs::write(index_path, json)
            .map_err(|e| AppError::StorageError(format!("Failed to write connections index: {}", e)))?;

        Ok(())
    }

    pub fn update_index_on_save(&self, connection: &Connection) -> AppResult<()> {
        let mut index = self.load_connection_index()?;

        let metadata = ConnectionMetadata {
            id: connection.id.clone(),
            name: connection.name.clone(),
        };

        if let Some(pos) = index.connections.iter().position(|c| c.id == connection.id) {
            index.connections[pos] = metadata;
        } else {
            index.connections.push(metadata);
        }

        self.save_connection_index(&index)
    }

    pub fn update_index_on_delete(&self, id: &str) -> AppResult<()> {
        let mut index = self.load_connection_index()?;
        index.connections.retain(|c| c.id != id);
        self.save_connection_index(&index)
    }

    pub fn get_connection_ids(&self) -> AppResult<Vec<String>> {
        let index = self.load_connection_index()?;
        Ok(index.connections.into_iter().map(|c| c.id).collect())
    }
}

// Stronghold commands that will be called from JavaScript
#[tauri::command]
pub async fn stronghold_save_connection<R: Runtime>(
    app: AppHandle<R>,
    connection: Connection,
) -> AppResult<()> {
    let storage = app.state::<crate::AppState>();
    let stronghold = storage.stronghold.lock().map_err(|e| {
        AppError::StorageError(format!("Failed to lock stronghold storage: {}", e))
    })?;

    stronghold.update_index_on_save(&connection)?;

    Ok(())
}

#[tauri::command]
pub async fn stronghold_delete_connection<R: Runtime>(
    app: AppHandle<R>,
    id: String,
) -> AppResult<()> {
    let storage = app.state::<crate::AppState>();
    let stronghold = storage.stronghold.lock().map_err(|e| {
        AppError::StorageError(format!("Failed to lock stronghold storage: {}", e))
    })?;

    stronghold.update_index_on_delete(&id)?;

    Ok(())
}

#[tauri::command]
pub async fn stronghold_get_connection_ids<R: Runtime>(
    app: AppHandle<R>,
) -> AppResult<Vec<String>> {
    let storage = app.state::<crate::AppState>();
    let stronghold = storage.stronghold.lock().map_err(|e| {
        AppError::StorageError(format!("Failed to lock stronghold storage: {}", e))
    })?;

    stronghold.get_connection_ids()
}
