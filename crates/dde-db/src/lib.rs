//! DocDamage Engine - Database Layer
//! 
//! SQLite-based persistence for world state, entities, and all game data.
//! The entire game project is a SQLite database file.

use std::path::Path;

use rusqlite::{Connection, Transaction};

pub mod migrations;
pub mod models;
pub mod queries;

pub use models::*;

/// Database error types
#[derive(thiserror::Error, Debug)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    
    #[error("Migration error: {0}")]
    Migration(String),
    
    #[error("Entity not found: {0}")]
    NotFound(u64),
    
    #[error("Invalid data: {0}")]
    InvalidData(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, DbError>;

/// Database wrapper with connection management
pub struct Database {
    conn: Connection,
    project_path: String,
}

impl Database {
    /// Create/open a project database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let conn = Connection::open(path)?;
        
        // Configure pragmas for performance
        conn.execute_batch(PRAGMAS)?;
        
        // Run migrations
        migrations::run_migrations(&conn)?;
        
        Ok(Self {
            conn,
            project_path: path.to_string_lossy().to_string(),
        })
    }
    
    /// Create a new project database
    pub fn create_new<P: AsRef<Path>>(path: P, project_name: &str) -> Result<Self> {
        let path = path.as_ref();
        
        // Remove existing file if present
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        
        let conn = Connection::open(path)?;
        
        // Configure pragmas
        conn.execute_batch(PRAGMAS)?;
        
        // Run migrations
        migrations::run_migrations(&conn)?;
        
        // Initialize project metadata
        let project_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp_millis();
        let seed = rand::random::<u64>();
        let _ = seed;
        
        conn.execute(
            "INSERT INTO project_meta (project_id, project_name, schema_version, world_seed, tick_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (&project_id, project_name, migrations::CURRENT_SCHEMA_VERSION, seed as i64, 0i64, now, now),
        )?;
        
        tracing::info!("Created new project '{}' at {:?}", project_name, path);
        
        Ok(Self {
            conn,
            project_path: path.to_string_lossy().to_string(),
        })
    }
    
    /// Get a reference to the connection
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
    
    /// Start a transaction
    pub fn transaction(&mut self) -> Result<Transaction> {
        Ok(self.conn.transaction()?)
    }
    
    /// Execute integrity check
    pub fn integrity_check(&self) -> Result<bool> {
        let valid: String = self.conn.query_row(
            "PRAGMA integrity_check",
            [],
            |row| row.get(0)
        )?;
        Ok(valid == "ok")
    }
    
    /// Get project metadata
    pub fn get_project_meta(&self) -> Result<ProjectMeta> {
        let meta = self.conn.query_row(
            "SELECT project_id, project_name, schema_version, world_seed, tick_count, created_at, updated_at FROM project_meta LIMIT 1",
            [],
            |row| {
                Ok(ProjectMeta {
                    project_id: row.get(0)?,
                    project_name: row.get(1)?,
                    schema_version: row.get(2)?,
                    world_seed: row.get::<_, i64>(3)? as u64,
                    tick_count: row.get::<_, i64>(4)? as u64,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            }
        )?;
        Ok(meta)
    }
    
    /// Save the database to a slot
    pub fn save_to_slot(&self, slot: u32) -> Result<()> {
        // TODO: Implement save slot logic
        tracing::info!("Saving to slot {}", slot);
        Ok(())
    }
    
    /// Get the project path
    pub fn path(&self) -> &str {
        &self.project_path
    }
}

/// SQLite pragmas for performance
const PRAGMAS: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = -64000;
PRAGMA mmap_size = 268435456;
PRAGMA temp_store = MEMORY;
"#;

/// Project metadata
#[derive(Debug, Clone)]
pub struct ProjectMeta {
    pub project_id: String,
    pub project_name: String,
    pub schema_version: i32,
    pub world_seed: u64,
    pub tick_count: u64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    fn test_db_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("dde_test_{}.db", uuid::Uuid::new_v4()));
        path
    }
    
    #[test]
    fn test_create_new_project() {
        let path = test_db_path();
        let db = Database::create_new(&path, "Test Project").unwrap();
        
        let meta = db.get_project_meta().unwrap();
        assert_eq!(meta.project_name, "Test Project");
        assert!(db.integrity_check().unwrap());
        
        // Cleanup
        drop(db);
        let _ = std::fs::remove_file(&path);
    }
    
    #[test]
    fn test_open_existing() {
        let path = test_db_path();
        
        // Create
        {
            let db = Database::create_new(&path, "Existing Project").unwrap();
            drop(db);
        }
        
        // Open
        {
            let db = Database::open(&path).unwrap();
            let meta = db.get_project_meta().unwrap();
            assert_eq!(meta.project_name, "Existing Project");
        }
        
        // Cleanup
        let _ = std::fs::remove_file(&path);
    }
}
