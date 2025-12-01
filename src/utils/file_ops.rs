use crate::error::{Error, Result};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a file change that can be tracked for potential rollback
#[derive(Debug, Clone)]
pub enum FileChange {
    /// File was created (contains its content for potential rollback)
    Created {
        original_existed: bool,
        original_content: Option<Vec<u8>>,
    },
    /// File was renamed (contains source path for potential rollback)
    Renamed { source_path: PathBuf },
}

impl FileChange {
    /// Creates a new FileChange for a created file
    pub fn new_created() -> Self {
        FileChange::Created {
            original_existed: false,
            original_content: None,
        }
    }

    /// Creates a new FileChange for a created file, storing original content for rollback
    pub fn created_with_content(content: Vec<u8>) -> Self {
        FileChange::Created {
            original_existed: true,
            original_content: Some(content),
        }
    }

    /// Creates a new FileChange for a renamed file
    pub fn renamed(source_path: PathBuf) -> Self {
        FileChange::Renamed { source_path }
    }
}

/// Tracks file changes and provides rollback functionality
pub struct FileTracker {
    /// Map of file paths to their tracked changes
    changes: HashMap<PathBuf, FileChange>,
    /// Whether automatic restore on drop is enabled
    restore_enabled: bool,
    /// Whether to force rollback regardless of restore_enabled
    force_rollback: bool,
}

impl Default for FileTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl FileTracker {
    /// Creates a new FileTracker with restore on drop enabled
    pub fn new() -> Self {
        Self {
            changes: HashMap::new(),
            restore_enabled: true,
            force_rollback: false,
        }
    }

    /// Creates a new FileTracker with restore on drop configurable
    pub fn new_with_restore(restore_enabled: bool) -> Self {
        Self {
            changes: HashMap::new(),
            restore_enabled,
            force_rollback: false,
        }
    }

    /// Starts tracking a file
    pub fn track_file(&mut self, path: &Path) -> Result<()> {
        debug!("Tracking file: {}", path.display());

        if self.changes.contains_key(path) {
            debug!("File already tracked: {}", path.display());
            return Ok(());
        }

        // If the file already exists, store its content for potential rollback
        if path.exists() {
            let content = fs::read(path).map_err(|e| Error::FileOperation {
                path: path.to_path_buf(),
                message: format!("Failed to read file content: {}", e),
            })?;

            self.changes.insert(
                path.to_path_buf(),
                FileChange::created_with_content(content),
            );
        } else {
            self.changes
                .insert(path.to_path_buf(), FileChange::new_created());
        }

        info!("Started tracking file: {}", path.display());
        Ok(())
    }

    /// Tracks a file rename operation
    pub fn track_rename(&mut self, source: &Path, target: &Path) -> Result<()> {
        debug!(
            "Tracking file rename: {} -> {}",
            source.display(),
            target.display()
        );

        if !source.exists() {
            return Err(Error::FileOperation {
                path: source.to_path_buf(),
                message: "Source file doesn't exist".to_string(),
            });
        }

        self.changes.insert(
            target.to_path_buf(),
            FileChange::renamed(source.to_path_buf()),
        );

        info!(
            "Tracked rename operation: {} -> {}",
            source.display(),
            target.display()
        );
        Ok(())
    }

    /// Force rollback of tracked changes
    pub fn force_rollback(&mut self) {
        self.force_rollback = true;
    }

    /// Rollback all tracked changes
    pub fn rollback(&mut self) -> Result<()> {
        info!("Rolling back file changes...");

        // Process file changes in reverse order
        let paths: Vec<PathBuf> = self.changes.keys().cloned().collect();
        for path in paths.iter().rev() {
            if let Some(change) = self.changes.get(path) {
                match change {
                    FileChange::Created {
                        original_existed,
                        original_content,
                    } => {
                        if *original_existed {
                            if let Some(content) = original_content {
                                fs::write(path, content).map_err(|e| Error::FileOperation {
                                    path: path.to_path_buf(),
                                    message: format!("Failed to restore file content: {}", e),
                                })?;
                                info!("Restored original content to {}", path.display());
                            }
                        } else if path.exists() {
                            fs::remove_file(path).map_err(|e| Error::FileOperation {
                                path: path.to_path_buf(),
                                message: format!("Failed to remove file: {}", e),
                            })?;
                            info!("Removed created file: {}", path.display());
                        }
                    }
                    FileChange::Renamed { source_path } => {
                        if path.exists() {
                            if source_path.exists() {
                                // Both files exist - this typically happens when:
                                // 1. Original file was renamed to backup (path)
                                // 2. Migration created new file at original location (source_path)
                                // 3. Rollback needs to restore original content
                                //
                                // We restore by copying content from backup to original, then removing backup.
                                debug!(
                                    "Both '{}' and '{}' exist during rollback. \
                                     Restoring original content from backup.",
                                    source_path.display(),
                                    path.display()
                                );
                                let content = fs::read(path).map_err(|e| Error::FileOperation {
                                    path: path.to_path_buf(),
                                    message: format!(
                                        "Failed to read backup file for rollback: {}",
                                        e
                                    ),
                                })?;
                                fs::write(source_path, content).map_err(|e| {
                                    Error::FileOperation {
                                        path: source_path.to_path_buf(),
                                        message: format!("Failed to restore original file: {}", e),
                                    }
                                })?;
                                fs::remove_file(path).map_err(|e| Error::FileOperation {
                                    path: path.to_path_buf(),
                                    message: format!("Failed to remove backup file: {}", e),
                                })?;
                            } else {
                                // Simple rename back
                                fs::rename(path, source_path).map_err(|e| {
                                    Error::FileOperation {
                                        path: path.to_path_buf(),
                                        message: format!(
                                            "Failed to rename back to {}: {}",
                                            source_path.display(),
                                            e
                                        ),
                                    }
                                })?;
                            }
                            info!(
                                "Renamed file back: {} -> {}",
                                path.display(),
                                source_path.display()
                            );
                        }
                    }
                }
            }
        }

        self.changes.clear();
        info!("Rollback completed successfully");
        Ok(())
    }

    /// Clear tracked changes without rollback
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.changes.clear();
    }
}

impl Drop for FileTracker {
    fn drop(&mut self) {
        // Only perform rollback if force_rollback is true and restore_enabled is true
        if self.force_rollback && self.restore_enabled && !self.changes.is_empty() {
            match self.rollback() {
                Ok(_) => {}
                Err(e) => {
                    warn!("Error during automatic rollback: {}", e);
                }
            }
        }
    }
}

/// A guard wrapper around FileTracker that simplifies working with tracked files
pub struct FileTrackerGuard {
    inner: FileTracker,
}

impl Default for FileTrackerGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl FileTrackerGuard {
    /// Creates a new FileTrackerGuard with restore on drop enabled
    pub fn new() -> Self {
        Self {
            inner: FileTracker::new(),
        }
    }

    /// Creates a new FileTrackerGuard with restore on drop configurable
    pub fn new_with_restore(restore_enabled: bool) -> Self {
        Self {
            inner: FileTracker::new_with_restore(restore_enabled),
        }
    }

    /// Starts tracking a file
    pub fn track_file(&mut self, path: &Path) -> Result<()> {
        self.inner.track_file(path)
    }

    /// Tracks a file rename operation
    pub fn track_rename(&mut self, source: &Path, target: &Path) -> Result<()> {
        self.inner.track_rename(source, target)
    }

    /// Force rollback of tracked changes
    pub fn force_rollback(&mut self) {
        self.inner.force_rollback();
    }
}
