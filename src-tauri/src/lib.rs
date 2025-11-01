mod error;
mod db;
mod ai;
mod storage;
mod import_export;

use error::AppResult;
use storage::{StorageManager, StrongholdStorage, AppSettings};
use db::connection::{Connection, ConnectionManager};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};
use chrono::Utc;

// Global state
pub struct AppState {
    storage: Mutex<StorageManager>,
    stronghold: Mutex<StrongholdStorage>,
    connections: Arc<ConnectionManager>,
}

// Settings Commands
#[tauri::command]
async fn save_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> AppResult<()> {
    let storage = state.storage.lock().map_err(|e| {
        error::AppError::StorageError(format!("Failed to lock storage: {}", e))
    })?;
    storage.save_settings(settings)
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> AppResult<Option<AppSettings>> {
    let storage = state.storage.lock().map_err(|e| {
        error::AppError::StorageError(format!("Failed to lock storage: {}", e))
    })?;
    storage.get_settings()
}

// Connection Commands
#[tauri::command]
async fn test_connection(
    state: State<'_, AppState>,
    connection: Connection,
) -> AppResult<serde_json::Value> {
    state.connections.test_connection(&connection).await?;

    Ok(serde_json::json!({
        "success": true,
        "message": "Connection successful"
    }))
}

#[tauri::command]
async fn save_connection(
    state: State<'_, AppState>,
    mut connection: Connection,
) -> AppResult<Connection> {
    // Generate ID and timestamps if new
    if connection.id.is_empty() {
        connection.id = uuid::Uuid::new_v4().to_string();
    }

    let now = Utc::now().to_rfc3339();
    if connection.created_at.is_empty() {
        connection.created_at = now.clone();
    }
    connection.updated_at = now;

    // Save to in-memory storage
    state.connections.save_connection(connection.clone())?;

    // Update metadata index for Stronghold
    let stronghold = state.stronghold.lock().map_err(|e| {
        error::AppError::StorageError(format!("Failed to lock stronghold storage: {}", e))
    })?;
    stronghold.update_index_on_save(&connection)?;

    Ok(connection)
}

#[tauri::command]
async fn get_connections(state: State<'_, AppState>) -> AppResult<Vec<Connection>> {
    state.connections.get_connections()
}

#[tauri::command]
async fn delete_connection(state: State<'_, AppState>, id: String) -> AppResult<()> {
    // Delete from in-memory storage
    state.connections.delete_connection(&id)?;

    // Update metadata index for Stronghold
    let stronghold = state.stronghold.lock().map_err(|e| {
        error::AppError::StorageError(format!("Failed to lock stronghold storage: {}", e))
    })?;
    stronghold.update_index_on_delete(&id)?;

    Ok(())
}

#[tauri::command]
async fn update_connection(
    state: State<'_, AppState>,
    mut connection: Connection,
) -> AppResult<Connection> {
    connection.updated_at = Utc::now().to_rfc3339();

    // Save to in-memory storage
    state.connections.save_connection(connection.clone())?;

    // Update metadata index for Stronghold
    let stronghold = state.stronghold.lock().map_err(|e| {
        error::AppError::StorageError(format!("Failed to lock stronghold storage: {}", e))
    })?;
    stronghold.update_index_on_save(&connection)?;

    Ok(connection)
}

// Schema Commands
#[tauri::command]
async fn get_schema(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    connection_id: String,
) -> AppResult<db::schema::Schema> {
    db::schema::get_schema(&state.connections, &connection_id, &app).await
}

#[tauri::command]
async fn run_query(
    state: State<'_, AppState>,
    connection_id: String,
    query: String,
    limit: i32,
    offset: i32,
) -> AppResult<db::query::QueryResult> {
    db::query::execute_query(&state.connections, &connection_id, &query, limit, offset).await
}

// Import/Export Commands
#[tauri::command]
async fn export_tables(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    options: import_export::export::ExportOptions,
) -> AppResult<String> {
    import_export::export::export_tables(app, &state.connections, options).await
}

#[tauri::command]
async fn import_tables(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    options: import_export::import::ImportOptions,
) -> AppResult<()> {
    import_export::import::import_tables(app, &state.connections, options).await
}

// AI Agent Commands
#[tauri::command]
async fn stream_ai_chat(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    message: String,
    connection_id: String,
) -> AppResult<()> {
    // Get settings
    let storage = state.storage.lock().map_err(|e| {
        error::AppError::StorageError(format!("Failed to lock storage: {}", e))
    })?;

    let settings = storage.get_settings()?.ok_or_else(|| {
        error::AppError::ConfigError("No settings found. Please configure OpenRouter API key.".into())
    })?;

    // Validate API key
    if settings.openrouter_api_key.is_empty() {
        return Err(error::AppError::ConfigError("OpenRouter API key not configured".into()));
    }

    drop(storage); // Release lock before async work

    // Run agent in background (non-blocking)
    let connections = Arc::clone(&state.connections);
    tokio::spawn(async move {
        let result = ai::run_react_agent(
            session_id.clone(),
            connection_id,
            message,
            &app,
            &connections,
            &settings,
        ).await;

        // Save conversation after agent completes
        if let Ok(response) = &result {
            // Load existing messages and append
            if let Ok(mut messages) = ai::load_conversation(&app, &session_id) {
                // In a real implementation, we'd track all messages from the agent run
                // For now, just save a marker
                let _ = ai::save_conversation(&app, &session_id, &session_id, &messages);
            }
        }

        if let Err(e) = result {
            eprintln!("Agent error: {}", e);
            // Emit error event to frontend
            let _ = app.emit("ai_error", serde_json::json!({
                "session_id": session_id,
                "error": e.to_string(),
            }));
        }
    });

    Ok(())
}

#[tauri::command]
async fn get_conversation_history(
    app: tauri::AppHandle,
    session_id: String,
) -> AppResult<Vec<ai::agent::Message>> {
    ai::load_conversation(&app, &session_id)
}

#[tauri::command]
async fn clear_conversation(
    app: tauri::AppHandle,
    session_id: String,
) -> AppResult<()> {
    ai::clear_conversation(&app, &session_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_stronghold::Builder::new(|password| {
            // Use Argon2id for password hashing with production-grade parameters
            use argon2::{
                password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
                Argon2, Params, Version,
            };

            // Generate a salt (in production, this could be derived from a machine-specific value)
            let salt = SaltString::generate(&mut OsRng);

            // Configure Argon2id with secure parameters
            let params = Params::new(
                19_456,  // 19 MB memory cost
                2,       // 2 iterations
                1,       // 1 thread of parallelism
                Some(32) // 32 byte output
            ).expect("Invalid Argon2 parameters");

            let argon2 = Argon2::new(
                argon2::Algorithm::Argon2id,
                Version::V0x13,
                params,
            );

            // Hash the password
            let password_hash = argon2
                .hash_password(password.as_ref(), &salt)
                .expect("Failed to hash password")
                .hash
                .expect("Missing hash output");

            password_hash.as_bytes().to_vec()
        }).build())
        .setup(|app| {
            let app_handle = app.handle();

            // Initialize storage
            let storage = StorageManager::new(app_handle)
                .expect("Failed to initialize storage");

            // Initialize Stronghold storage
            let stronghold = StrongholdStorage::new(app_handle)
                .expect("Failed to initialize Stronghold storage");

            let connection_manager = Arc::new(ConnectionManager::new());

            // Store in app state
            app.manage(AppState {
                storage: Mutex::new(storage),
                stronghold: Mutex::new(stronghold),
                connections: connection_manager,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            save_settings,
            get_settings,
            test_connection,
            save_connection,
            get_connections,
            delete_connection,
            update_connection,
            get_schema,
            run_query,
            export_tables,
            import_tables,
            stream_ai_chat,
            get_conversation_history,
            clear_conversation,
            storage::stronghold::stronghold_save_connection,
            storage::stronghold::stronghold_delete_connection,
            storage::stronghold::stronghold_get_connection_ids,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
