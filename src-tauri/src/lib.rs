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

    // Persist full connection data to Stronghold
    let stronghold = state.stronghold.lock().map_err(|e| {
        error::AppError::StorageError(format!("Failed to lock stronghold storage: {}", e))
    })?;
    stronghold.save_connection(&connection)?;

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

    // Delete persisted connection data from Stronghold
    let stronghold = state.stronghold.lock().map_err(|e| {
        error::AppError::StorageError(format!("Failed to lock stronghold storage: {}", e))
    })?;
    stronghold.delete_connection(&id)?;

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

    // Persist full connection data to Stronghold
    let stronghold = state.stronghold.lock().map_err(|e| {
        error::AppError::StorageError(format!("Failed to lock stronghold storage: {}", e))
    })?;
    stronghold.save_connection(&connection)?;

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
async fn get_sql_keywords(
    state: State<'_, AppState>,
    connection_id: String,
) -> AppResult<Vec<db::keywords::SqlKeyword>> {
    db::keywords::fetch_keywords_from_pool(&state.connections, &connection_id).await
}

#[tauri::command]
async fn highlight_sql(
    sql: String,
    config: db::syntax_highlight::HighlightConfig,
) -> AppResult<String> {
    Ok(db::syntax_highlight::highlight_sql(&sql, &config))
}

#[tauri::command]
async fn run_query(
    state: State<'_, AppState>,
    connection_id: String,
    query: String,
    limit: i32,
    offset: i32,
) -> AppResult<db::query::QueryResult> {
    let start = std::time::Instant::now();
    let result = db::query::execute_query(&state.connections, &connection_id, &query, limit, offset).await;
    let execution_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Save to history
    let success = result.is_ok();
    let _ = storage::query_history::add_query_to_history(
        query,
        connection_id,
        execution_time_ms,
        success,
    ).await;

    result
}

#[tauri::command]
async fn get_query_history(connection_id: Option<String>) -> AppResult<Vec<storage::query_history::QueryHistoryEntry>> {
    storage::query_history::get_query_history(connection_id).await
}

#[tauri::command]
async fn clear_query_history() -> AppResult<()> {
    storage::query_history::clear_query_history().await
}

#[tauri::command]
async fn delete_query_from_history(query_id: String) -> AppResult<()> {
    storage::query_history::delete_query_from_history(query_id).await
}

#[tauri::command]
async fn commit_data_changes(
    state: State<'_, AppState>,
    request: db::commit::CommitRequest,
) -> AppResult<db::commit::CommitResult> {
    db::commit::commit_data_changes(&state.connections, request).await
}

#[tauri::command]
async fn clear_data_only(
    state: State<'_, AppState>,
    connection_id: String,
) -> AppResult<()> {
    db::clear::clear_data_only(&state.connections, &connection_id).await
}

#[tauri::command]
async fn clear_database(
    state: State<'_, AppState>,
    connection_id: String,
) -> AppResult<()> {
    db::clear::clear_database(&state.connections, &connection_id).await
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
async fn cancel_export(connection_id: String) -> AppResult<()> {
    import_export::export::cancel_export(connection_id).await
}

#[tauri::command]
async fn import_tables(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    options: import_export::import::ImportOptions,
) -> AppResult<()> {
    import_export::import::import_tables(app, &state.connections, options).await
}

#[tauri::command]
async fn cancel_import(connection_id: String) -> AppResult<()> {
    import_export::import::cancel_import(connection_id).await
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
    let history_limit = settings.conversation_history_limit;
    tokio::spawn(async move {
        // Load conversation history with limit
        let previous_messages = ai::load_conversation_with_limit(
            &app,
            &session_id,
            history_limit,
        )
        .unwrap_or_else(|e| {
            eprintln!("Failed to load conversation history: {}", e);
            Vec::new()
        });

        let result = ai::run_react_agent(
            session_id.clone(),
            connection_id.clone(),
            message.clone(),
            previous_messages.clone(),
            &app,
            &connections,
            &settings,
        ).await;

        // Save conversation after agent completes
        if let Ok(response) = &result {
            // Load all existing messages (not limited)
            let mut all_messages = ai::load_conversation(&app, &session_id)
                .unwrap_or_else(|_| Vec::new());

            // Append user message
            all_messages.push(ai::agent::Message::user(&message));

            // Append assistant response
            all_messages.push(ai::agent::Message::assistant(&response.answer));

            // Save complete conversation
            let _ = ai::save_conversation(&app, &session_id, &connection_id, &all_messages);
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

            // Initialize query history path
            let app_data_dir = app_handle
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            storage::query_history::init_history_path(app_data_dir);

            // Initialize storage
            let storage = StorageManager::new(app_handle)
                .expect("Failed to initialize storage");

            // Initialize Stronghold storage
            let stronghold = StrongholdStorage::new(app_handle)
                .expect("Failed to initialize Stronghold storage");

            let connection_manager = Arc::new(ConnectionManager::new());

            // Load persisted connections from stronghold
            match stronghold.load_all_connections() {
                Ok(connections) => {
                    for conn in connections {
                        if let Err(e) = connection_manager.save_connection(conn) {
                            eprintln!("Failed to restore connection: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load connections from storage: {}", e);
                }
            }

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
            get_sql_keywords,
            highlight_sql,
            run_query,
            get_query_history,
            clear_query_history,
            delete_query_from_history,
            commit_data_changes,
            clear_data_only,
            clear_database,
            export_tables,
            cancel_export,
            import_tables,
            cancel_import,
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
