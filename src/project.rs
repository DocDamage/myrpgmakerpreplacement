//! Project file handling
//!
//! Creates, opens, and saves .dde SQLite project files.

use std::path::{Path, PathBuf};

use dde_db::Database;
use tracing::info;

/// Project manager
pub struct ProjectManager {
    current_project: Option<Database>,
    project_path: Option<PathBuf>,
}

#[allow(dead_code)]
impl ProjectManager {
    /// Create a new project manager
    pub fn new() -> Self {
        Self {
            current_project: None,
            project_path: None,
        }
    }

    /// Create a new project
    pub fn create_new<P: AsRef<Path>>(&mut self, path: P, name: &str) -> anyhow::Result<()> {
        let path = path.as_ref();
        info!("Creating new project '{}' at {:?}", name, path);

        let db = Database::create_new(path, name)?;

        self.project_path = Some(path.to_path_buf());
        self.current_project = Some(db);

        info!("Project created successfully");
        Ok(())
    }

    /// Open an existing project
    pub fn open<P: AsRef<Path>>(&mut self, path: P) -> anyhow::Result<()> {
        let path = path.as_ref();
        info!("Opening project at {:?}", path);

        if !path.exists() {
            anyhow::bail!("Project file not found: {:?}", path);
        }

        let db = Database::open(path)?;

        // Verify integrity
        if !db.integrity_check()? {
            anyhow::bail!("Database integrity check failed");
        }

        // Get project metadata
        let meta = db.get_project_meta()?;
        info!(
            "Project: {} (schema v{})",
            meta.project_name, meta.schema_version
        );
        info!("World seed: {}", meta.world_seed);
        info!("Tick count: {}", meta.tick_count);

        self.project_path = Some(path.to_path_buf());
        self.current_project = Some(db);

        info!("Project opened successfully");
        Ok(())
    }

    /// Save the current project
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(ref _db) = self.current_project {
            info!("Saving project...");
            // SQLite handles this automatically with WAL mode
            // In the future, we might want to checkpoint WAL
            info!("Project saved");
            Ok(())
        } else {
            anyhow::bail!("No project is currently open");
        }
    }

    /// Save project to a specific path (Save As)
    pub fn save_as<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let path = path.as_ref();
        info!("Saving project to {:?}", path);

        if let Some(ref db) = self.current_project {
            // Copy the database file
            let source_path = db.path();
            std::fs::copy(source_path, path)?;
            info!("Project saved to {:?}", path);
            Ok(())
        } else {
            anyhow::bail!("No project is currently open");
        }
    }

    /// Close the current project
    pub fn close(&mut self) {
        if self.current_project.is_some() {
            info!("Closing project");
            self.current_project = None;
            self.project_path = None;
        }
    }

    /// Check if a project is open
    pub fn has_project(&self) -> bool {
        self.current_project.is_some()
    }

    /// Get the current project path
    pub fn project_path(&self) -> Option<&Path> {
        self.project_path.as_deref()
    }

    /// Get the current project name
    pub fn project_name(&self) -> Option<String> {
        self.current_project
            .as_ref()
            .and_then(|db| db.get_project_meta().ok())
            .map(|meta| meta.project_name)
    }

    /// Get a reference to the current database
    pub fn database(&self) -> Option<&Database> {
        self.current_project.as_ref()
    }

    /// Get a mutable reference to the current database
    pub fn database_mut(&mut self) -> Option<&mut Database> {
        self.current_project.as_mut()
    }
}

impl Default for ProjectManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a default project path
pub fn default_project_path() -> PathBuf {
    let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    path.push("projects");
    std::fs::create_dir_all(&path).ok();
    path
}

/// Generate a project filename from a name
pub fn project_filename(name: &str) -> String {
    format!("{}.dde", name.to_lowercase().replace(" ", "_"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_create_and_open_project() {
        let mut manager = ProjectManager::new();
        let test_path = temp_dir().join(format!("test_project_{}.dde", std::process::id()));

        // Create project
        manager.create_new(&test_path, "Test Project").unwrap();
        assert!(manager.has_project());
        assert_eq!(manager.project_name(), Some("Test Project".to_string()));

        // Close and reopen
        manager.close();
        assert!(!manager.has_project());

        manager.open(&test_path).unwrap();
        assert!(manager.has_project());
        assert_eq!(manager.project_name(), Some("Test Project".to_string()));

        // Cleanup
        manager.close();
        let _ = std::fs::remove_file(&test_path);
    }
}
