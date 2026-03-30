//! Hot Reload Watcher
//!
//! Enhanced file watching with debouncing and batch processing

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::asset_hot_reload::{AssetChangeEvent, AssetType, ChangeType};

/// Enhanced watcher with better debouncing and batching
pub struct HotReloadWatcher {
    watcher: RecommendedWatcher,
    pending_changes: Arc<Mutex<Vec<AssetChangeEvent>>>,
    tracked_paths: HashSet<PathBuf>,
    config: WatcherConfig,
    last_batch_time: Instant,
}

/// Configuration for the watcher
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Debounce duration for batching changes
    pub debounce_ms: u64,
    /// Maximum batch size before forcing processing
    pub max_batch_size: usize,
    /// Enable recursive watching
    pub recursive: bool,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 100,
            max_batch_size: 50,
            recursive: true,
        }
    }
}

/// Batched change set for efficient processing
#[derive(Debug, Clone)]
pub struct ChangeBatch {
    pub changes: Vec<AssetChangeEvent>,
    pub timestamp: Instant,
}

impl HotReloadWatcher {
    /// Create a new watcher
    pub fn new() -> crate::asset_hot_reload::Result<Self> {
        let pending_changes = Arc::new(Mutex::new(Vec::new()));
        let pending_clone = Arc::clone(&pending_changes);

        let watcher = RecommendedWatcher::new(
            move |res: std::result::Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    Self::handle_event(event, &pending_clone);
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(100)),
        )?;

        Ok(Self {
            watcher,
            pending_changes,
            tracked_paths: HashSet::new(),
            config: WatcherConfig::default(),
            last_batch_time: Instant::now(),
        })
    }

    /// Create with custom config
    pub fn with_config(config: WatcherConfig) -> crate::asset_hot_reload::Result<Self> {
        let mut watcher = Self::new()?;
        watcher.config = config;
        Ok(watcher)
    }

    /// Watch a directory or file
    pub fn watch(&mut self, path: &Path) -> crate::asset_hot_reload::Result<()> {
        let mode = if self.config.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        self.watcher.watch(path, mode)?;
        self.tracked_paths.insert(path.to_path_buf());
        
        tracing::info!("Hot reload watching: {:?}", path);
        Ok(())
    }

    /// Stop watching a path
    pub fn unwatch(&mut self, path: &Path) -> crate::asset_hot_reload::Result<()> {
        self.watcher.unwatch(path)?;
        self.tracked_paths.remove(path);
        Ok(())
    }

    /// Get pending changes as a batch if debounce period has passed
    pub fn get_batch(&mut self) -> Option<ChangeBatch> {
        let now = Instant::now();
        let debounce = Duration::from_millis(self.config.debounce_ms);

        // Check if we should process
        let should_process = {
            let pending = self.pending_changes.lock().ok()?;
            let elapsed = now.duration_since(self.last_batch_time);
            
            // Process if debounce passed AND we have changes
            // OR if we hit max batch size
            (elapsed >= debounce && !pending.is_empty())
                || pending.len() >= self.config.max_batch_size
        };

        if should_process {
            let changes = self.drain_pending();
            if !changes.is_empty() {
                self.last_batch_time = now;
                return Some(ChangeBatch {
                    changes,
                    timestamp: now,
                });
            }
        }

        None
    }

    /// Force immediate batch processing
    pub fn force_batch(&mut self) -> ChangeBatch {
        let changes = self.drain_pending();
        self.last_batch_time = Instant::now();
        
        ChangeBatch {
            changes,
            timestamp: Instant::now(),
        }
    }

    /// Check if there are pending changes
    pub fn has_pending(&self) -> bool {
        self.pending_changes
            .lock()
            .map(|p| !p.is_empty())
            .unwrap_or(false)
    }

    /// Get count of pending changes
    pub fn pending_count(&self) -> usize {
        self.pending_changes
            .lock()
            .map(|p| p.len())
            .unwrap_or(0)
    }

    /// Drain pending changes
    fn drain_pending(&mut self) -> Vec<AssetChangeEvent> {
        // Deduplicate by path, keeping most recent
        let mut latest_by_path: HashMap<PathBuf, AssetChangeEvent> = HashMap::new();

        if let Ok(mut pending) = self.pending_changes.lock() {
            for event in pending.drain(..) {
                latest_by_path.insert(event.path.clone(), event);
            }
        }

        // Convert back to vec, sorted by timestamp
        let mut changes: Vec<_> = latest_by_path.into_values().collect();
        changes.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        changes
    }

    /// Handle notify events
    fn handle_event(event: Event, pending: &Arc<Mutex<Vec<AssetChangeEvent>>>) {
        let change_type = match event.kind {
            notify::EventKind::Create(_) => ChangeType::Created,
            notify::EventKind::Modify(_) => ChangeType::Modified,
            notify::EventKind::Remove(_) => ChangeType::Deleted,
            _ => return,
        };

        for path in event.paths {
            if let Some(asset_type) = AssetType::from_path(&path) {
                let change_event = AssetChangeEvent {
                    path,
                    asset_type,
                    change_type: change_type.clone(),
                    timestamp: Instant::now(),
                };

                if let Ok(mut pending_guard) = pending.lock() {
                    pending_guard.push(change_event);
                }
            }
        }
    }
}

impl Drop for HotReloadWatcher {
    fn drop(&mut self) {
        // Watcher cleanup is automatic
        tracing::debug!("Hot reload watcher dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_watcher_config_default() {
        let config = WatcherConfig::default();
        assert_eq!(config.debounce_ms, 100);
        assert_eq!(config.max_batch_size, 50);
        assert!(config.recursive);
    }

    #[test]
    fn test_change_batch_creation() {
        let batch = ChangeBatch {
            changes: vec![],
            timestamp: Instant::now(),
        };
        assert!(batch.changes.is_empty());
    }
}
