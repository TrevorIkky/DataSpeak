use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sqlx::{MySqlPool, PgPool, Pool, Postgres, MySql};
use std::collections::HashMap;
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

    pub async fn test_connection(&self, conn: &Connection) -> AppResult<()> {
        match conn.database_type {
            DatabaseType::PostgreSQL => {
                let url = format!(
                    "postgresql://{}:{}@{}:{}/{}",
                    conn.username, conn.password, conn.host, conn.port, conn.default_database
                );
                let pool = PgPool::connect(&url).await?;

                // Test the connection
                sqlx::query("SELECT 1").fetch_one(&pool).await?;

                pool.close().await;
                Ok(())
            }
            DatabaseType::MariaDB | DatabaseType::MySQL => {
                let url = format!(
                    "mysql://{}:{}@{}:{}/{}",
                    conn.username, conn.password, conn.host, conn.port, conn.default_database
                );
                let pool = MySqlPool::connect(&url).await?;

                // Test the connection
                sqlx::query("SELECT 1").fetch_one(&pool).await?;

                pool.close().await;
                Ok(())
            }
        }
    }

    pub async fn get_pool_postgres(&self, connection_id: &str) -> AppResult<Pool<Postgres>> {
        // Check if pool exists
        {
            let pools = self.postgres_pools.lock().map_err(|e| {
                AppError::ConnectionError(format!("Failed to lock postgres pools: {}", e))
            })?;

            if let Some(pool) = pools.get(connection_id) {
                return Ok(pool.clone());
            }
        } // Lock is dropped here

        // Get connection details
        let url = {
            let connections = self.connections.lock().map_err(|e| {
                AppError::ConnectionError(format!("Failed to lock connections: {}", e))
            })?;

            let conn = connections
                .iter()
                .find(|c| c.id == connection_id)
                .ok_or_else(|| AppError::ConnectionError("Connection not found".to_string()))?;

            format!(
                "postgresql://{}:{}@{}:{}/{}",
                conn.username, conn.password, conn.host, conn.port, conn.default_database
            )
        }; // Lock is dropped here

        let pool = PgPool::connect(&url).await?;

        // Store the pool
        {
            let mut pools = self.postgres_pools.lock().map_err(|e| {
                AppError::ConnectionError(format!("Failed to lock postgres pools: {}", e))
            })?;
            pools.insert(connection_id.to_string(), pool.clone());
        } // Lock is dropped here

        Ok(pool)
    }

    pub async fn get_pool_mysql(&self, connection_id: &str) -> AppResult<Pool<MySql>> {
        // Check if pool exists
        {
            let pools = self.mysql_pools.lock().map_err(|e| {
                AppError::ConnectionError(format!("Failed to lock mysql pools: {}", e))
            })?;

            if let Some(pool) = pools.get(connection_id) {
                return Ok(pool.clone());
            }
        } // Lock is dropped here

        // Get connection details
        let url = {
            let connections = self.connections.lock().map_err(|e| {
                AppError::ConnectionError(format!("Failed to lock connections: {}", e))
            })?;

            let conn = connections
                .iter()
                .find(|c| c.id == connection_id)
                .ok_or_else(|| AppError::ConnectionError("Connection not found".to_string()))?;

            format!(
                "mysql://{}:{}@{}:{}/{}",
                conn.username, conn.password, conn.host, conn.port, conn.default_database
            )
        }; // Lock is dropped here

        let pool = MySqlPool::connect(&url).await?;

        // Store the pool
        {
            let mut pools = self.mysql_pools.lock().map_err(|e| {
                AppError::ConnectionError(format!("Failed to lock mysql pools: {}", e))
            })?;
            pools.insert(connection_id.to_string(), pool.clone());
        } // Lock is dropped here

        Ok(pool)
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
