use crate::error::{AppError, AppResult};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use sqlx::{MySqlPool, PgPool, Pool, Postgres, MySql};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub database_type: DatabaseType,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub default_database: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum DatabaseType {
    PostgreSQL,
    MariaDB,
    MySQL,
}

impl DatabaseType {
    /// Get the display name for the database type
    pub fn display_name(&self) -> &'static str {
        match self {
            DatabaseType::PostgreSQL => "PostgreSQL",
            DatabaseType::MySQL => "MySQL",
            DatabaseType::MariaDB => "MariaDB",
        }
    }
}

pub struct ConnectionManager {
    postgres_pools: Mutex<HashMap<String, Pool<Postgres>>>,
    mysql_pools: Mutex<HashMap<String, Pool<MySql>>>,
    connections: Mutex<Vec<Connection>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            postgres_pools: Mutex::new(HashMap::new()),
            mysql_pools: Mutex::new(HashMap::new()),
            connections: Mutex::new(Vec::new()),
        }
    }

    /// Build a properly encoded connection URL for the given connection.
    /// Handles special characters in username, password, and database name.
    fn build_connection_url(conn: &Connection) -> String {
        // Percent-encode credentials to handle special characters like @, :, /, etc.
        let username = utf8_percent_encode(&conn.username, NON_ALPHANUMERIC).to_string();
        let password = utf8_percent_encode(&conn.password, NON_ALPHANUMERIC).to_string();
        let database = utf8_percent_encode(&conn.default_database, NON_ALPHANUMERIC).to_string();

        match conn.database_type {
            DatabaseType::PostgreSQL => format!(
                "postgresql://{}:{}@{}:{}/{}",
                username, password, conn.host, conn.port, database
            ),
            DatabaseType::MariaDB | DatabaseType::MySQL => format!(
                "mysql://{}:{}@{}:{}/{}",
                username, password, conn.host, conn.port, database
            ),
        }
    }

    pub async fn test_connection(&self, conn: &Connection) -> AppResult<()> {
        let url = Self::build_connection_url(conn);

        match conn.database_type {
            DatabaseType::PostgreSQL => {
                let pool = PgPool::connect(&url).await?;
                sqlx::query("SELECT 1").fetch_one(&pool).await?;
                pool.close().await;
                Ok(())
            }
            DatabaseType::MariaDB | DatabaseType::MySQL => {
                let pool = MySqlPool::connect(&url).await?;
                sqlx::query("SELECT 1").fetch_one(&pool).await?;
                pool.close().await;
                Ok(())
            }
        }
    }

    pub async fn get_pool_postgres(&self, connection_id: &str) -> AppResult<Pool<Postgres>> {
        // Fast path: check if pool already exists
        {
            let pools = self.postgres_pools.lock().map_err(|e| {
                AppError::ConnectionError(format!("Failed to lock postgres pools: {}", e))
            })?;

            if let Some(pool) = pools.get(connection_id) {
                return Ok(pool.clone());
            }
        }

        // Get connection details and build URL (outside of lock)
        let url = {
            let connections = self.connections.lock().map_err(|e| {
                AppError::ConnectionError(format!("Failed to lock connections: {}", e))
            })?;

            let conn = connections
                .iter()
                .find(|c| c.id == connection_id)
                .ok_or_else(|| AppError::ConnectionError("Connection not found".to_string()))?;

            Self::build_connection_url(conn)
        };

        // Connect outside of lock to avoid blocking other operations
        let pool = PgPool::connect(&url).await?;

        // Use entry API to handle race condition gracefully
        // If another thread created the pool while we were connecting,
        // we'll use their pool and drop ours
        let mut pools = self.postgres_pools.lock().map_err(|e| {
            AppError::ConnectionError(format!("Failed to lock postgres pools: {}", e))
        })?;

        Ok(match pools.entry(connection_id.to_string()) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => entry.insert(pool).clone(),
        })
    }

    pub async fn get_pool_mysql(&self, connection_id: &str) -> AppResult<Pool<MySql>> {
        // Fast path: check if pool already exists
        {
            let pools = self.mysql_pools.lock().map_err(|e| {
                AppError::ConnectionError(format!("Failed to lock mysql pools: {}", e))
            })?;

            if let Some(pool) = pools.get(connection_id) {
                return Ok(pool.clone());
            }
        }

        // Get connection details and build URL (outside of lock)
        let url = {
            let connections = self.connections.lock().map_err(|e| {
                AppError::ConnectionError(format!("Failed to lock connections: {}", e))
            })?;

            let conn = connections
                .iter()
                .find(|c| c.id == connection_id)
                .ok_or_else(|| AppError::ConnectionError("Connection not found".to_string()))?;

            Self::build_connection_url(conn)
        };

        // Connect outside of lock to avoid blocking other operations
        let pool = MySqlPool::connect(&url).await?;

        // Use entry API to handle race condition gracefully
        let mut pools = self.mysql_pools.lock().map_err(|e| {
            AppError::ConnectionError(format!("Failed to lock mysql pools: {}", e))
        })?;

        Ok(match pools.entry(connection_id.to_string()) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => entry.insert(pool).clone(),
        })
    }

    pub fn save_connection(&self, conn: Connection) -> AppResult<Connection> {
        let mut connections = self.connections.lock().map_err(|e| {
            AppError::ConnectionError(format!("Failed to lock connections: {}", e))
        })?;

        // Check if connection with same ID exists
        if let Some(index) = connections.iter().position(|c| c.id == conn.id) {
            connections[index] = conn.clone();
        } else {
            connections.push(conn.clone());
        }

        Ok(conn)
    }

    pub fn get_connections(&self) -> AppResult<Vec<Connection>> {
        let connections = self.connections.lock().map_err(|e| {
            AppError::ConnectionError(format!("Failed to lock connections: {}", e))
        })?;

        Ok(connections.clone())
    }

    pub fn delete_connection(&self, id: &str) -> AppResult<()> {
        let mut connections = self.connections.lock().map_err(|e| {
            AppError::ConnectionError(format!("Failed to lock connections: {}", e))
        })?;

        connections.retain(|c| c.id != id);

        // Remove pools
        let mut pg_pools = self.postgres_pools.lock().map_err(|e| {
            AppError::ConnectionError(format!("Failed to lock postgres pools: {}", e))
        })?;
        pg_pools.remove(id);

        let mut mysql_pools = self.mysql_pools.lock().map_err(|e| {
            AppError::ConnectionError(format!("Failed to lock mysql pools: {}", e))
        })?;
        mysql_pools.remove(id);

        Ok(())
    }

    pub fn get_connection(&self, id: &str) -> AppResult<Connection> {
        let connections = self.connections.lock().map_err(|e| {
            AppError::ConnectionError(format!("Failed to lock connections: {}", e))
        })?;

        connections
            .iter()
            .find(|c| c.id == id)
            .cloned()
            .ok_or_else(|| AppError::ConnectionError("Connection not found".to_string()))
    }
}
