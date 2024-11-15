use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use uv_migrator::migrators::poetry::PoetryMigrationSource;
use uv_migrator::migrators::{DependencyType, MigrationSource};

/// Helper function to create a temporary test project with a pyproject.toml file.
///
/// # Arguments
///
/// * `content` - The content to write to the pyproject.toml file
///
/// # Returns
///
/// A tuple containing the temporary directory and its path
fn create_test_project(content: &str) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().to_path_buf();
    let pyproject_path = project_dir.join("pyproject.toml");
    fs::write(&pyproject_path, content).unwrap();
    (temp_dir, project_dir)
}

/// Test that main dependencies are correctly extracted from a Poetry project.
///
/// This test verifies that:
/// 1. Python version requirements are excluded
/// 2. Regular dependencies are parsed with correct versions
/// 3. Dependencies with table definitions (extras) are handled properly
/// 4. All dependencies are marked as main dependencies
#[test]
fn test_extract_main_dependencies() {
    let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.11"
fastapi = "^0.111.0"
aiofiles = "24.1.0"
jinja2 = { version = "^3.1.4" }
uvicorn = { extras = ["standard"], version = "^0.30.1" }
"#;
    let (_temp_dir, project_dir) = create_test_project(content);

    let source = PoetryMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 4); // Should not include python

    let fastapi_dep = dependencies.iter().find(|d| d.name == "fastapi").unwrap();
    assert_eq!(fastapi_dep.version, Some("^0.111.0".to_string()));
    assert_eq!(fastapi_dep.dep_type, DependencyType::Main);

    let aiofiles_dep = dependencies.iter().find(|d| d.name == "aiofiles").unwrap();
    assert_eq!(aiofiles_dep.version, Some("24.1.0".to_string()));
    assert_eq!(aiofiles_dep.dep_type, DependencyType::Main);
}

/// Test that development dependencies are correctly extracted from a Poetry project.
///
/// This test verifies that:
/// 1. Dependencies in the dev group are correctly identified
/// 2. All dev dependencies have the correct DependencyType::Dev
/// 3. Version specifications are properly parsed
#[test]
fn test_extract_dev_dependencies() {
    let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.11"

[tool.poetry.group.dev.dependencies]
pytest = "^8.2.2"
pytest-cov = "^5.0.0"
pytest-sugar = "^1.0.0"
"#;
    let (_temp_dir, project_dir) = create_test_project(content);

    let source = PoetryMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 3);

    let pytest_dep = dependencies.iter().find(|d| d.name == "pytest").unwrap();
    assert_eq!(pytest_dep.version, Some("^8.2.2".to_string()));
    assert_eq!(pytest_dep.dep_type, DependencyType::Dev);
}

/// Test handling of multiple dependency groups in a Poetry project.
///
/// This test verifies that:
/// 1. Main dependencies are correctly identified
/// 2. Multiple dev groups (dev, code-quality) are handled properly
/// 3. All non-main dependencies are marked as dev dependencies
/// 4. Dependencies from different groups maintain their version specifications
#[test]
fn test_extract_multiple_groups() {
    let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.11"
fastapi = "^0.111.0"

[tool.poetry.group.dev.dependencies]
pytest = "^8.2.2"

[tool.poetry.group.code-quality.dependencies]
ruff = "^0.5.0"
mypy = "^1.11.1"
"#;
    let (_temp_dir, project_dir) = create_test_project(content);

    let source = PoetryMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 4); // 1 main + 1 dev + 2 code-quality

    // Check main dependencies
    let main_deps: Vec<_> = dependencies
        .iter()
        .filter(|d| matches!(d.dep_type, DependencyType::Main))
        .collect();
    assert_eq!(main_deps.len(), 1);
    assert_eq!(main_deps[0].name, "fastapi");

    // Check dev dependencies
    let dev_deps: Vec<_> = dependencies
        .iter()
        .filter(|d| matches!(d.dep_type, DependencyType::Dev))
        .collect();
    assert_eq!(dev_deps.len(), 1);
    assert_eq!(dev_deps[0].name, "pytest");

    // Check code-quality group dependencies
    let code_quality_deps: Vec<_> = dependencies
        .iter()
        .filter(|d| matches!(d.dep_type, DependencyType::Group(ref g) if g == "code-quality"))
        .collect();
    assert_eq!(code_quality_deps.len(), 2);
    assert!(code_quality_deps.iter().any(|d| d.name == "ruff"));
    assert!(code_quality_deps.iter().any(|d| d.name == "mypy"));
}

/// Test handling of dependencies with extras (optional features) in a Poetry project.
///
/// This test verifies that:
/// 1. Dependencies with extras are parsed correctly
/// 2. Version specifications within table definitions are extracted properly
/// 3. Python version requirements are still excluded
#[test]
fn test_handle_dependency_with_extras() {
    let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.11"
uvicorn = { extras = ["standard"], version = "^0.30.1" }
aiohttp = { extras = ["speedups"], version = "^3.10.5" }
"#;
    let (_temp_dir, project_dir) = create_test_project(content);

    let source = PoetryMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 2);

    let uvicorn_dep = dependencies.iter().find(|d| d.name == "uvicorn").unwrap();
    assert_eq!(uvicorn_dep.version, Some("^0.30.1".to_string()));
}

/// Test handling of dependencies without version specifications.
///
/// This test verifies that:
/// 1. Dependencies with "*" version are parsed as having no version constraint
/// 2. Python version requirements are excluded
/// 3. The resulting dependency list contains only the expected items
#[test]
fn test_handle_missing_version() {
    let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.11"
requests = "*"
"#;
    let (_temp_dir, project_dir) = create_test_project(content);

    let source = PoetryMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 1);

    let requests_dep = dependencies.iter().find(|d| d.name == "requests").unwrap();
    assert_eq!(requests_dep.version, None);
}

/// Test error handling for invalid TOML syntax in pyproject.toml.
///
/// This test verifies that:
/// 1. Invalid TOML content results in an error
/// 2. The error message contains appropriate information about parsing failure
#[test]
fn test_error_invalid_toml() {
    let content = r#"
[tool.poetry
name = "test-project"
"#;
    let (_temp_dir, project_dir) = create_test_project(content);

    let source = PoetryMigrationSource;
    let result = source.extract_dependencies(&project_dir);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Error parsing TOML"));
}

/// Test error handling when pyproject.toml is missing.
///
/// This test verifies that:
/// 1. Attempting to extract dependencies from a non-existent file results in an error
/// 2. The error message contains appropriate information about the missing file
#[test]
fn test_error_missing_file() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().to_path_buf();

    let source = PoetryMigrationSource;
    let result = source.extract_dependencies(&project_dir);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Error reading file"));
}

/// Test handling of test group dependencies in a Poetry project.
///
/// This test verifies that:
/// 1. Dependencies in the test group are correctly identified
/// 2. Version specifications are properly parsed
/// 3. Test group dependencies are marked as dev dependencies
#[test]
fn test_extract_test_group_dependencies() {
    let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.11"

[tool.poetry.group.test.dependencies]
blue = ">=0.9.1"
"#;
    let (_temp_dir, project_dir) = create_test_project(content);

    let source = PoetryMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 1);

    let blue_dep = dependencies.iter().find(|d| d.name == "blue").unwrap();
    assert_eq!(blue_dep.version, Some(">=0.9.1".to_string()));
    assert_eq!(blue_dep.dep_type, DependencyType::Dev); // All group dependencies should be marked as Dev
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;
    use uv_migrator::utils::update_pyproject_toml;

    /// Helper function to create a temporary test directory.
    ///
    /// # Returns
    ///
    /// A temporary directory that will be automatically cleaned up when dropped
    fn setup_test_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp directory")
    }

    /// Test migration of project metadata from Poetry to uv format.
    ///
    /// This test verifies that:
    /// 1. The version is correctly migrated from old to new pyproject.toml
    /// 2. The description is properly transferred from Poetry format
    /// 3. The metadata is correctly formatted in the new pyproject.toml
    ///
    /// # Returns
    ///
    /// A Result indicating whether the test passed or failed with an error message
    #[test]
    fn test_description_migration() -> Result<(), String> {
        let test_dir = setup_test_dir();

        // Create old.pyproject.toml with description
        let old_content = r#"[tool.poetry]
name = "test-project"
version = "1.3.0"
description = "a module for parsing battlescribe rosters and allowing them to be printed or displayed cleanly"
authors = ["Test Author <test@example.com>"]
license = "MIT"
"#;
        fs::write(test_dir.path().join("old.pyproject.toml"), old_content)
            .map_err(|e| format!("Failed to write old.pyproject.toml: {}", e))?;

        // Create new pyproject.toml with default content
        let new_content = r#"[project]
name = "test-project"
version = "0.1.0"
description = "Add your description here"
"#;
        fs::write(test_dir.path().join("pyproject.toml"), new_content)
            .map_err(|e| format!("Failed to write pyproject.toml: {}", e))?;

        // Run the migration
        update_pyproject_toml(test_dir.path(), &[])?;

        // Read the result
        let result = fs::read_to_string(test_dir.path().join("pyproject.toml"))
            .map_err(|e| format!("Failed to read result: {}", e))?;

        // Verify the changes
        assert!(
            result.contains(r#"version = "1.3.0""#),
            "Version was not updated correctly"
        );
        assert!(result.contains(r#"description = "a module for parsing battlescribe rosters and allowing them to be printed or displayed cleanly""#),
                "Description was not updated correctly");

        Ok(())
    }

    /// Test handling of missing description in Poetry project file.
    ///
    /// This test verifies that:
    /// 1. The version is correctly migrated from old to new pyproject.toml
    /// 2. The default description remains unchanged when no description exists in Poetry file
    /// 3. Other metadata fields are properly updated
    ///
    /// # Returns
    ///
    /// A Result indicating whether the test passed or failed with an error message
    #[test]
    fn test_missing_description() -> Result<(), String> {
        let test_dir = setup_test_dir();

        // Create old.pyproject.toml without description
        let old_content = r#"[tool.poetry]
name = "test-project"
version = "1.3.0"
authors = ["Test Author <test@example.com>"]
license = "MIT"
"#;
        fs::write(test_dir.path().join("old.pyproject.toml"), old_content)
            .map_err(|e| format!("Failed to write old.pyproject.toml: {}", e))?;

        // Create new pyproject.toml with default content
        let new_content = r#"[project]
name = "test-project"
version = "0.1.0"
description = "Add your description here"
"#;
        fs::write(test_dir.path().join("pyproject.toml"), new_content)
            .map_err(|e| format!("Failed to write pyproject.toml: {}", e))?;

        // Run the migration
        update_pyproject_toml(test_dir.path(), &[])?;

        // Read the result
        let result = fs::read_to_string(test_dir.path().join("pyproject.toml"))
            .map_err(|e| format!("Failed to read result: {}", e))?;

        // Verify the changes
        assert!(
            result.contains(r#"version = "1.3.0""#),
            "Version was not updated correctly"
        );
        assert!(
            result.contains(r#"description = "Add your description here""#),
            "Description should remain unchanged when not present in Poetry file"
        );

        Ok(())
    }

    /// Test behavior when old.pyproject.toml is missing.
    ///
    /// This test verifies that:
    /// 1. The migration process handles missing old.pyproject.toml gracefully
    /// 2. The existing pyproject.toml remains unchanged
    /// 3. No errors are thrown when the old configuration file is absent
    ///
    /// # Returns
    ///
    /// A Result indicating whether the test passed or failed with an error message
    #[test]
    fn test_no_old_pyproject() -> Result<(), String> {
        let test_dir = setup_test_dir();

        // Create only new pyproject.toml with default content
        let new_content = r#"[project]
name = "test-project"
version = "0.1.0"
description = "Add your description here"
"#;
        fs::write(test_dir.path().join("pyproject.toml"), new_content)
            .map_err(|e| format!("Failed to write pyproject.toml: {}", e))?;

        // Run the migration
        update_pyproject_toml(test_dir.path(), &[])?;

        // Read the result
        let result = fs::read_to_string(test_dir.path().join("pyproject.toml"))
            .map_err(|e| format!("Failed to read result: {}", e))?;

        // Verify nothing changed
        assert_eq!(
            result, new_content,
            "File should remain unchanged when no old.pyproject.toml exists"
        );

        Ok(())
    }
}
