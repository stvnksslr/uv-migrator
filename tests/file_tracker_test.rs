use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use uv_migrator::utils::FileTrackerGuard;

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a temporary test environment with directory and file.
    ///
    /// Returns:
    /// - TempDir: The temporary directory handle (automatically cleaned up when dropped)
    /// - PathBuf: Path to the project directory
    /// - PathBuf: Path to a test file within the project directory
    fn setup_test_environment() -> (TempDir, PathBuf, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();
        let test_file = project_dir.join("test.txt");
        fs::write(&test_file, "test content").unwrap();
        (temp_dir, project_dir, test_file)
    }

    /// Tests that a new file can be tracked successfully.
    ///
    /// This test verifies that:
    /// 1. A file can be added to tracking
    /// 2. The tracking operation completes without errors
    #[test]
    fn test_track_new_file() {
        let (_temp_dir, _project_dir, test_file) = setup_test_environment();
        let mut guard = FileTrackerGuard::new();
        let result = guard.track_file(&test_file);
        assert!(result.is_ok());
    }

    /// Tests that tracking the same file twice is idempotent.
    ///
    /// This test verifies that:
    /// 1. A file can be tracked multiple times
    /// 2. Subsequent tracking of the same file doesn't cause errors
    #[test]
    fn test_track_same_file_twice() {
        let (_temp_dir, _project_dir, test_file) = setup_test_environment();
        let mut guard = FileTrackerGuard::new();
        
        assert!(guard.track_file(&test_file).is_ok());
        assert!(guard.track_file(&test_file).is_ok());
    }

    /// Tests file rename tracking functionality.
    ///
    /// This test verifies that:
    /// 1. A file rename operation can be tracked
    /// 2. The tracking completes successfully
    #[test]
    fn test_track_rename() {
        let (_temp_dir, project_dir, test_file) = setup_test_environment();
        let new_path = project_dir.join("renamed.txt");
        let mut guard = FileTrackerGuard::new();
        
        let result = guard.track_rename(&test_file, &new_path);
        assert!(result.is_ok());
    }

    /// Tests handling of rename operations with nonexistent source files.
    ///
    /// This test verifies that:
    /// 1. Attempting to track a rename of a nonexistent file results in an error
    /// 2. The error message correctly indicates the file doesn't exist
    #[test]
    fn test_track_rename_nonexistent_file() {
        let (_temp_dir, project_dir, _test_file) = setup_test_environment();
        let nonexistent = project_dir.join("nonexistent.txt");
        let new_path = project_dir.join("renamed.txt");
        let mut guard = FileTrackerGuard::new();
        
        let result = guard.track_rename(&nonexistent, &new_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    /// Tests automatic rollback functionality of FileTrackerGuard.
    ///
    /// This test verifies that:
    /// 1. When force_rollback is called, the guard restores files
    /// 2. Files are restored to their original state
    /// 3. The rollback occurs when the guard is dropped
    #[test]
    fn test_file_tracker_guard_auto_rollback() {
        let (_temp_dir, project_dir, _) = setup_test_environment();
        let pyproject_path = project_dir.join("pyproject.toml");
        let backup_path = project_dir.join("old.pyproject.toml");
        
        // Create initial pyproject.toml
        fs::write(&pyproject_path, "original content").unwrap();
        
        {
            let mut guard = FileTrackerGuard::new();
            guard.track_rename(&pyproject_path, &backup_path).unwrap();
            fs::rename(&pyproject_path, &backup_path).unwrap();
            
            // Force rollback
            guard.force_rollback();
        } // Guard is dropped here
        
        // Verify original file is restored
        assert!(pyproject_path.exists());
        let content = fs::read_to_string(&pyproject_path).unwrap();
        assert_eq!(content, "original content");
    }

    /// Tests that rollback properly restores files to their original state.
    ///
    /// This test verifies that:
    /// 1. Files are properly backed up during rename operations
    /// 2. Original content is preserved
    /// 3. Rollback restores both the file and its content
    #[test]
    fn test_rollback_restores_files() {
        let (_temp_dir, project_dir, _) = setup_test_environment();
        let pyproject_path = project_dir.join("pyproject.toml");
        let backup_path = project_dir.join("old.pyproject.toml");
        
        // Create and track initial pyproject.toml
        fs::write(&pyproject_path, "original content").unwrap();
        
        {
            let mut guard = FileTrackerGuard::new();
            guard.track_rename(&pyproject_path, &backup_path).unwrap();
            fs::rename(&pyproject_path, &backup_path).unwrap();
            fs::write(&pyproject_path, "new content").unwrap();
            guard.force_rollback();
        } // Guard is dropped here
        
        assert!(pyproject_path.exists());
        let content = fs::read_to_string(&pyproject_path).unwrap();
        assert_eq!(content, "original content");
    }

    /// Tests handling of files in nested directories.
    ///
    /// This test verifies that:
    /// 1. Files in nested directories can be tracked
    /// 2. Parent directories are properly handled
    /// 3. Tracking works with deep directory structures
    #[test]
    fn test_nested_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested").join("dir").join("file.txt");
        
        // Create parent directories first
        if let Some(parent) = nested_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&nested_path, "test content").unwrap();
        
        let mut guard = FileTrackerGuard::new();
        assert!(guard.track_file(&nested_path).is_ok());
    }
    
    /// Tests tracking of multiple file operations.
    ///
    /// This test verifies that:
    /// 1. Multiple files can be tracked simultaneously
    /// 2. Different operations (track and rename) can be mixed
    /// 3. All operations complete successfully
    #[test]
    fn test_multiple_operations() {
        let (_temp_dir, project_dir, _) = setup_test_environment();
        let mut guard = FileTrackerGuard::new();
        
        let file1 = project_dir.join("file1.txt");
        let file2 = project_dir.join("file2.txt");
        let file3 = project_dir.join("file3.txt");
        
        fs::write(&file1, "content1").unwrap();
        fs::write(&file2, "content2").unwrap();
        
        // Track multiple files
        assert!(guard.track_file(&file1).is_ok());
        assert!(guard.track_file(&file2).is_ok());
        
        // Perform a rename
        assert!(guard.track_rename(&file1, &file3).is_ok());
    }
}