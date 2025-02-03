use log::{debug, error, info};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum FileAction {
    Created,
    Renamed {
        source_path: PathBuf,
        source_content: Vec<u8>,
    },
}

#[derive(Debug)]
pub struct FileChange {
    pub action: FileAction,
}

pub struct FileTracker {
    pub(crate) changes: HashMap<PathBuf, FileChange>,
}

impl Default for FileTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl FileTracker {
    pub fn new() -> Self {
        FileTracker {
            changes: HashMap::new(),
        }
    }

    pub fn track_file(&mut self, path: &Path) -> Result<(), String> {
        debug!("Attempting to track file: {}", path.display());
        if self.changes.contains_key(path) {
            debug!("File already being tracked: {}", path.display());
            return Ok(());
        }
        self.changes.insert(
            path.to_path_buf(),
            FileChange {
                action: FileAction::Created,
            },
        );
        info!("Successfully started tracking file: {}", path.display());
        Ok(())
    }

    pub fn track_rename(&mut self, from: &Path, to: &Path) -> Result<(), String> {
        debug!(
            "Attempting to track rename from '{}' to '{}'",
            from.display(),
            to.display()
        );
        if !from.exists() {
            return Err(format!("Source file '{}' does not exist", from.display()));
        }
        let source_content = fs::read(from).map_err(|e| {
            format!(
                "Failed to read source file '{}' for tracking: {}",
                from.display(),
                e
            )
        })?;
        self.changes.insert(
            to.to_path_buf(),
            FileChange {
                action: FileAction::Renamed {
                    source_path: from.to_path_buf(),
                    source_content,
                },
            },
        );
        info!(
            "Successfully tracked rename operation: '{}' → '{}'",
            from.display(),
            to.display()
        );
        Ok(())
    }

    pub(crate) fn ensure_parent_dir_exists(path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                debug!("Creating parent directory: {}", parent.display());
                fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "Failed to create parent directory '{}': {}",
                        parent.display(),
                        e
                    )
                })?;
            }
        }
        Ok(())
    }

    pub fn rollback(&self) -> Result<(), String> {
        if self.changes.is_empty() {
            info!("No changes to roll back");
            return Ok(());
        }
        info!("Starting rollback sequence");
        let pyproject_path = Path::new("pyproject.toml");
        if pyproject_path.exists() {
            info!("Phase 1: Removing current pyproject.toml");
            fs::remove_file(pyproject_path)
                .map_err(|e| format!("Failed to remove pyproject.toml: {}", e))?;
        }
        info!("Phase 2: Restoring original pyproject.toml");
        for change in self.changes.values() {
            if let FileAction::Renamed {
                source_path,
                source_content,
            } = &change.action
            {
                if source_path.ends_with("pyproject.toml") {
                    Self::ensure_parent_dir_exists(source_path)?;
                    fs::write(source_path, source_content)
                        .map_err(|e| format!("Failed to restore pyproject.toml: {}", e))?;
                    if !source_path.exists() {
                        return Err("Failed to verify restored pyproject.toml".to_string());
                    }
                    info!("Successfully restored pyproject.toml");
                    return Ok(());
                }
            }
        }
        Err("Could not find original pyproject.toml to restore".to_string())
    }
}

pub struct FileTrackerGuard {
    tracker: FileTracker,
    should_rollback: bool,
    has_performed_rollback: bool,
    restore_enabled: bool,
}

impl Default for FileTrackerGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl FileTrackerGuard {
    pub fn new() -> Self {
        FileTrackerGuard {
            tracker: FileTracker::new(),
            should_rollback: false,
            has_performed_rollback: false,
            restore_enabled: true,
        }
    }

    pub fn new_with_restore(restore_enabled: bool) -> Self {
        FileTrackerGuard {
            tracker: FileTracker::new(),
            should_rollback: false,
            has_performed_rollback: false,
            restore_enabled,
        }
    }

    pub fn track_file(&mut self, path: &Path) -> Result<(), String> {
        self.tracker.track_file(path)
    }

    pub fn track_rename(&mut self, from: &Path, to: &Path) -> Result<(), String> {
        self.tracker.track_rename(from, to)
    }

    pub fn force_rollback(&mut self) {
        self.should_rollback = true;
    }

    fn perform_rollback(&mut self) {
        if !self.has_performed_rollback && self.restore_enabled {
            if let Err(e) = self.tracker.rollback() {
                error!("Error during rollback: {}", e);
            }
            self.has_performed_rollback = true;
        }
    }
}

impl Drop for FileTrackerGuard {
    fn drop(&mut self) {
        if (std::thread::panicking() || self.should_rollback) && self.restore_enabled {
            self.perform_rollback();
        }
    }
}
