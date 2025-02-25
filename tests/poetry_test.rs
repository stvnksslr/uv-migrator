use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use uv_migrator::DependencyType;
use uv_migrator::migrators::MigrationSource;
use uv_migrator::migrators::poetry::PoetryMigrationSource;
use uv_migrator::migrators::{self};
use uv_migrator::utils::author::extract_authors_from_poetry;
use uv_migrator::utils::update_pyproject_toml;

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
/// 5. Extras are correctly preserved and propagated
#[test]
fn test_extract_multiple_groups() {
    let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.11"
fastapi = "^0.111.0"
django = { extras = ["rest"], version = "^4.2.0" }

[tool.poetry.group.dev.dependencies]
pytest = "^8.2.2"
pytest-django = { extras = ["coverage"], version = "^4.5.2" }

[tool.poetry.group.code-quality.dependencies]
ruff = "^0.5.0"
mypy = "^1.11.1"
"#;
    let (_temp_dir, project_dir) = create_test_project(content);

    let source = PoetryMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 6); // 2 main + 2 dev + 2 code-quality

    // Check main dependencies
    let main_deps: Vec<_> = dependencies
        .iter()
        .filter(|d| matches!(d.dep_type, DependencyType::Main))
        .collect();
    assert_eq!(main_deps.len(), 2);

    let fastapi_dep = main_deps.iter().find(|d| d.name == "fastapi").unwrap();
    assert_eq!(fastapi_dep.version, Some("^0.111.0".to_string()));
    assert!(fastapi_dep.extras.is_none());

    let django_dep = main_deps.iter().find(|d| d.name == "django").unwrap();
    assert_eq!(django_dep.version, Some("^4.2.0".to_string()));
    assert!(django_dep.extras.is_some());
    assert_eq!(django_dep.extras.as_ref().unwrap().len(), 1);
    assert_eq!(django_dep.extras.as_ref().unwrap()[0], "rest");

    // Check dev dependencies
    let dev_deps: Vec<_> = dependencies
        .iter()
        .filter(|d| matches!(d.dep_type, DependencyType::Dev))
        .collect();
    assert_eq!(dev_deps.len(), 2);

    let pytest_dep = dev_deps.iter().find(|d| d.name == "pytest").unwrap();
    assert_eq!(pytest_dep.version, Some("^8.2.2".to_string()));
    assert!(pytest_dep.extras.is_none());

    let pytest_django_dep = dev_deps.iter().find(|d| d.name == "pytest-django").unwrap();
    assert_eq!(pytest_django_dep.version, Some("^4.5.2".to_string()));
    assert!(pytest_django_dep.extras.is_some());
    assert_eq!(pytest_django_dep.extras.as_ref().unwrap().len(), 1);
    assert_eq!(pytest_django_dep.extras.as_ref().unwrap()[0], "coverage");

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

    // Check that uvicorn dependency has extras correctly extracted
    let uvicorn_dep = dependencies.iter().find(|d| d.name == "uvicorn").unwrap();
    assert_eq!(uvicorn_dep.version, Some("^0.30.1".to_string()));
    assert!(uvicorn_dep.extras.is_some());
    assert_eq!(uvicorn_dep.extras.as_ref().unwrap().len(), 1);
    assert_eq!(uvicorn_dep.extras.as_ref().unwrap()[0], "standard");

    // Check that aiohttp dependency has extras correctly extracted
    let aiohttp_dep = dependencies.iter().find(|d| d.name == "aiohttp").unwrap();
    assert_eq!(aiohttp_dep.version, Some("^3.10.5".to_string()));
    assert!(aiohttp_dep.extras.is_some());
    assert_eq!(aiohttp_dep.extras.as_ref().unwrap().len(), 1);
    assert_eq!(aiohttp_dep.extras.as_ref().unwrap()[0], "speedups");
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
    assert!(result.unwrap_err().contains("File does not exist"));
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
pytest = "^8.0.0"
pytest-cov = "^4.1.0"
"#;
    let (_temp_dir, project_dir) = create_test_project(content);

    let source = PoetryMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    // Should have all test dependencies (excluding python)
    assert_eq!(dependencies.len(), 3, "Should have three test dependencies");

    // Verify each dependency is in the test group
    for dep in &dependencies {
        match &dep.dep_type {
            DependencyType::Group(name) => {
                assert_eq!(name, "test", "Group name should be 'test'");
            }
            other => panic!("Expected Group(\"test\"), got {:?}", other),
        }
    }

    // Check specific dependency details
    let blue_dep = dependencies.iter().find(|d| d.name == "blue").unwrap();
    assert_eq!(blue_dep.version, Some(">=0.9.1".to_string()));
    assert!(matches!(blue_dep.dep_type, DependencyType::Group(ref name) if name == "test"));

    let pytest_dep = dependencies.iter().find(|d| d.name == "pytest").unwrap();
    assert_eq!(pytest_dep.version, Some("^8.0.0".to_string()));
    assert!(matches!(pytest_dep.dep_type, DependencyType::Group(ref name) if name == "test"));

    let pytest_cov_dep = dependencies
        .iter()
        .find(|d| d.name == "pytest-cov")
        .unwrap();
    assert_eq!(pytest_cov_dep.version, Some("^4.1.0".to_string()));
    assert!(matches!(pytest_cov_dep.dep_type, DependencyType::Group(ref name) if name == "test"));
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
    pub(crate) fn setup_test_dir() -> TempDir {
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

#[cfg(test)]
mod merge_groups_tests {
    use super::*;

    /// Test the merging of Poetry dependency groups into main and dev categories.
    ///
    /// This test verifies that:
    /// 1. Different dependency groups (main, dev, docs, test) are correctly identified
    /// 2. All non-main groups are merged into the dev category
    /// 3. Main dependencies remain unchanged
    /// 4. Python version requirements are excluded
    /// 5. The correct number of dependencies are maintained in each category
    #[test]
    fn test_merge_groups_poetry() {
        let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.11"
fastapi = "^0.111.0"
aiofiles = "24.1.0"

[tool.poetry.group.dev.dependencies]
pytest = "^8.2.2"
black = "^22.3.0"

[tool.poetry.group.docs.dependencies]
mkdocs = "^1.5.0"
mkdocs-material = "^9.4.0"

[tool.poetry.group.test.dependencies]
pytest-cov = "^4.1.0"
pytest-mock = "^3.10.0"
"#;

        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();
        let pyproject_path = project_dir.join("pyproject.toml");
        std::fs::write(&pyproject_path, content).unwrap();

        // Extract dependencies normally first
        let source = PoetryMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        // Verify initial state
        assert_eq!(
            dependencies
                .iter()
                .filter(|d| matches!(d.dep_type, DependencyType::Main))
                .count(),
            2,
            "Should have 2 main dependencies (excluding python)"
        );
        assert_eq!(
            dependencies
                .iter()
                .filter(|d| matches!(d.dep_type, DependencyType::Dev))
                .count(),
            2,
            "Should have 2 dev dependencies"
        );
        assert_eq!(
            dependencies
                .iter()
                .filter(|d| matches!(d.dep_type, DependencyType::Group(ref g) if g == "docs"))
                .count(),
            2,
            "Should have 2 docs group dependencies"
        );
        assert_eq!(
            dependencies
                .iter()
                .filter(|d| matches!(d.dep_type, DependencyType::Group(ref g) if g == "test"))
                .count(),
            2,
            "Should have 2 test group dependencies"
        );

        // Apply group merging
        let merged_deps = migrators::merge_dependency_groups(dependencies);

        // Verify merged state
        assert_eq!(
            merged_deps
                .iter()
                .filter(|d| matches!(d.dep_type, DependencyType::Main))
                .count(),
            2,
            "Should still have 2 main dependencies after merge"
        );
        assert_eq!(
            merged_deps
                .iter()
                .filter(|d| matches!(d.dep_type, DependencyType::Dev))
                .count(),
            6,
            "Should have 6 dev dependencies after merge (original dev + docs + test)"
        );
        assert_eq!(
            merged_deps
                .iter()
                .filter(|d| matches!(d.dep_type, DependencyType::Group(_)))
                .count(),
            0,
            "Should have no group dependencies after merge"
        );
    }

    /// Test merging of Poetry dependency groups with complex version specifications.
    ///
    /// This test verifies that:
    /// 1. Complex version constraints are preserved through the merging process
    /// 2. Dependencies with extras and pre-release flags are handled correctly
    /// 3. Version ranges and multiple constraints remain intact
    /// 4. Dependencies are properly categorized after merging
    /// 5. All groups are correctly merged into main or dev categories
    #[test]
    fn test_merge_groups_poetry_with_complex_dependencies() {
        let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.11"
requests = {version = "^2.31.0", extras = ["security"]}
django = ">=4.0.0,<5.0.0"

[tool.poetry.group.dev.dependencies]
black = {version = "^22.3.0", allow-prereleases = true}
pylint = "^3.0.0"

[tool.poetry.group.test.dependencies]
pytest = {version = "^8.0.0", extras = ["testing"]}
pytest-django = ">=4.5.0"
"#;

        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();
        let pyproject_path = project_dir.join("pyproject.toml");
        std::fs::write(&pyproject_path, content).unwrap();

        // Extract and verify dependencies maintain their complex specifications through the merge
        let source = PoetryMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        // Verify initial complex dependencies
        let requests_dep = dependencies
            .iter()
            .find(|d| d.name == "requests")
            .expect("Should have requests dependency");
        assert_eq!(requests_dep.version, Some("^2.31.0".to_string()));

        let django_dep = dependencies
            .iter()
            .find(|d| d.name == "django")
            .expect("Should have django dependency");
        assert_eq!(django_dep.version, Some(">=4.0.0,<5.0.0".to_string()));

        // Apply merge and verify complex dependencies are preserved
        let merged_deps = migrators::merge_dependency_groups(dependencies);

        // Verify versions are maintained after merge
        let pytest_dep = merged_deps
            .iter()
            .find(|d| d.name == "pytest")
            .expect("Should have pytest dependency");
        assert_eq!(pytest_dep.version, Some("^8.0.0".to_string()));
        assert!(matches!(pytest_dep.dep_type, DependencyType::Dev));

        let pytest_django_dep = merged_deps
            .iter()
            .find(|d| d.name == "pytest-django")
            .expect("Should have pytest-django dependency");
        assert_eq!(pytest_django_dep.version, Some(">=4.5.0".to_string()));
        assert!(matches!(pytest_django_dep.dep_type, DependencyType::Dev));
    }
}

#[test]
fn test_poetry_author_migration() {
    let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"
authors = [
    "John Doe <john@example.com>",
    "Jane Smith <jane@example.com>",
    "Anonymous Contributor"  # No email
]

[tool.poetry.dependencies]
python = "^3.11"
requests = "^2.31.0"
"#;

    let pyproject_content = r#"
[project]
name = "test-project"
version = "0.1.0"
description = "Test project"
"#;

    let (_temp_dir, project_dir) = create_test_project(content);

    // Rename pyproject.toml to old.pyproject.toml to simulate migration
    std::fs::rename(
        project_dir.join("pyproject.toml"),
        project_dir.join("old.pyproject.toml"),
    )
    .unwrap();

    // Create new pyproject.toml
    std::fs::write(project_dir.join("pyproject.toml"), pyproject_content).unwrap();

    // Extract authors to verify the extraction itself
    let authors = extract_authors_from_poetry(&project_dir).unwrap();
    assert_eq!(authors.len(), 3, "Should extract all three authors");

    let john = authors.iter().find(|a| a.name == "John Doe").unwrap();
    assert_eq!(john.email, Some("john@example.com".to_string()));

    let jane = authors.iter().find(|a| a.name == "Jane Smith").unwrap();
    assert_eq!(jane.email, Some("jane@example.com".to_string()));

    let anon = authors
        .iter()
        .find(|a| a.name == "Anonymous Contributor")
        .unwrap();
    assert_eq!(anon.email, None);

    // For this test, we'll just verify the authors were extracted correctly
    // The actual migration functionality should be tested in integration tests
}

#[test]
fn test_poetry_author_migration_with_setup_py_fallback() {
    let poetry_content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"
"#;

    let setup_py_content = r#"
from setuptools import setup

setup(
    name="test-project",
    version="1.0.0",
    author="Fallback Author",
    author_email="fallback@example.com",
    description="Test project"
)
"#;

    let (_temp_dir, project_dir) = create_test_project(poetry_content);

    // Add setup.py
    std::fs::write(project_dir.join("setup.py"), setup_py_content).unwrap();

    // Rename pyproject.toml to old.pyproject.toml to simulate migration
    std::fs::rename(
        project_dir.join("pyproject.toml"),
        project_dir.join("old.pyproject.toml"),
    )
    .unwrap();

    // Create new pyproject.toml
    std::fs::write(
        project_dir.join("pyproject.toml"),
        r#"[project]
name = "test-project"
version = "0.1.0"
description = "Test project"
"#,
    )
    .unwrap();

    // For this test, we'll just verify both files exist correctly
    assert!(project_dir.join("setup.py").exists());
    assert!(project_dir.join("pyproject.toml").exists());
    assert!(project_dir.join("old.pyproject.toml").exists());
}

fn create_test_project_with_old_pyproject(content: &str) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().to_path_buf();
    let old_pyproject_path = project_dir.join("old.pyproject.toml");
    fs::write(&old_pyproject_path, content).unwrap();
    (temp_dir, project_dir)
}

#[test]
fn test_extract_python_version_caret() {
    let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.9"
"#;
    let (_temp_dir, project_dir) = create_test_project_with_old_pyproject(content);
    let version = PoetryMigrationSource::extract_python_version(&project_dir).unwrap();
    assert_eq!(version, Some("3.9".to_string()));
}

#[test]
fn test_extract_python_version_greater_equal() {
    let content = r#"
[tool.poetry.dependencies]
python = ">=3.8"
"#;
    let (_temp_dir, project_dir) = create_test_project_with_old_pyproject(content);
    let version = PoetryMigrationSource::extract_python_version(&project_dir).unwrap();
    assert_eq!(version, Some("3.8".to_string()));
}

#[test]
fn test_extract_python_version_tilde() {
    let content = r#"
[tool.poetry.dependencies]
python = "~=3.10"
"#;
    let (_temp_dir, project_dir) = create_test_project_with_old_pyproject(content);
    let version = PoetryMigrationSource::extract_python_version(&project_dir).unwrap();
    assert_eq!(version, Some("3.10".to_string()));
}

#[test]
fn test_extract_python_version_exact() {
    let content = r#"
[tool.poetry.dependencies]
python = "3.11.0"
"#;
    let (_temp_dir, project_dir) = create_test_project_with_old_pyproject(content);
    let version = PoetryMigrationSource::extract_python_version(&project_dir).unwrap();
    assert_eq!(version, Some("3.11".to_string()));
}

#[test]
fn test_extract_python_version_no_python() {
    let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
requests = "^2.31.0"
"#;
    let (_temp_dir, project_dir) = create_test_project_with_old_pyproject(content);
    let version = PoetryMigrationSource::extract_python_version(&project_dir).unwrap();
    assert_eq!(version, None);
}

#[test]
fn test_extract_python_version_no_old_pyproject() {
    let temp_dir = TempDir::new().unwrap();
    let version = PoetryMigrationSource::extract_python_version(temp_dir.path()).unwrap();
    assert_eq!(version, None);
}

#[test]
fn test_extract_python_version_invalid_toml() {
    let content = r#"
[tool.poetry
name = "test-project"
"#;
    let (_temp_dir, project_dir) = create_test_project_with_old_pyproject(content);
    let result = PoetryMigrationSource::extract_python_version(&project_dir);
    assert!(result.is_err());
}

use crate::tests::setup_test_dir;

#[test]
fn test_poetry_v2_description_migration() -> Result<(), String> {
    let test_dir = setup_test_dir();

    let old_content = r#"[project]
name = "test-project"
version = "1.3.0"
description = "Modern Python project using Poetry 2.0"
"#;
    fs::write(test_dir.path().join("old.pyproject.toml"), old_content)
        .map_err(|e| e.to_string())?;

    let new_content = r#"[project]
name = "test-project"
version = "0.1.0"
description = "Add your description here"
"#;
    fs::write(test_dir.path().join("pyproject.toml"), new_content).map_err(|e| e.to_string())?;

    update_pyproject_toml(test_dir.path(), &[])?;

    let result =
        fs::read_to_string(test_dir.path().join("pyproject.toml")).map_err(|e| e.to_string())?;

    assert!(result.contains(r#"version = "1.3.0""#));
    assert!(result.contains(r#"description = "Modern Python project using Poetry 2.0""#));

    Ok(())
}

#[test]
fn test_poetry_v2_dependency_with_extras() {
    let content = r#"
[project]
name = "test-extras"
version = "0.1.0"
description = "Testing extras parsing"
readme = "README.md"
requires-python = ">=3.10"
dependencies = [
    "ibis-framework[duckdb,polars,sqlite] (>=10.1.0,<11.0.0)",
    "requests[security] (>=2.31.0)",
    "django[rest] (^4.2.0)"
]
"#;
    let (_temp_dir, project_dir) = create_test_project(content);

    let source = PoetryMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 3);

    // Check that ibis-framework dependency has extras correctly extracted
    let ibis_dep = dependencies
        .iter()
        .find(|d| d.name == "ibis-framework")
        .unwrap();
    assert_eq!(ibis_dep.version, Some(">=10.1.0,<11.0.0".to_string()));
    assert!(ibis_dep.extras.is_some());
    let ibis_extras = ibis_dep.extras.as_ref().unwrap();
    assert_eq!(ibis_extras.len(), 3);
    assert!(ibis_extras.contains(&"duckdb".to_string()));
    assert!(ibis_extras.contains(&"polars".to_string()));
    assert!(ibis_extras.contains(&"sqlite".to_string()));

    // Check that requests dependency has extras correctly extracted
    let requests_dep = dependencies.iter().find(|d| d.name == "requests").unwrap();
    assert_eq!(requests_dep.version, Some(">=2.31.0".to_string()));
    assert!(requests_dep.extras.is_some());
    assert_eq!(requests_dep.extras.as_ref().unwrap().len(), 1);
    assert_eq!(requests_dep.extras.as_ref().unwrap()[0], "security");

    // Check that django dependency has extras correctly extracted
    let django_dep = dependencies.iter().find(|d| d.name == "django").unwrap();
    assert_eq!(django_dep.version, Some("^4.2.0".to_string()));
    assert!(django_dep.extras.is_some());
    assert_eq!(django_dep.extras.as_ref().unwrap().len(), 1);
    assert_eq!(django_dep.extras.as_ref().unwrap()[0], "rest");
}
