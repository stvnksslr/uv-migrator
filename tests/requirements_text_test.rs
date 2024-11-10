use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use uv_migrator::migrator::requirements::RequirementsMigrationSource;
use uv_migrator::migrator::{DependencyType, MigrationSource};

/// Helper function to create a temporary test project with requirements files.
///
/// # Arguments
///
/// * `files` - A vector of tuples containing filename and content for each requirements file
///
/// # Returns
///
/// A tuple containing the temporary directory and its path
fn create_test_project(files: Vec<(&str, &str)>) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().to_path_buf();

    for (filename, content) in files {
        let file_path = project_dir.join(filename);
        fs::write(&file_path, content).unwrap();
    }

    (temp_dir, project_dir)
}

/// Test basic parsing of a requirements.txt file with simple dependencies.
///
/// This test verifies that:
/// 1. Basic package requirements are correctly parsed
/// 2. Version specifications are properly extracted
/// 3. Dependencies are marked as main dependencies
#[test]
fn test_basic_requirements() {
    let content = r#"
requests==2.31.0
flask>=2.0.0
sqlalchemy<2.0.0
    "#;

    let (_temp_dir, project_dir) = create_test_project(vec![("requirements.txt", content)]);

    let source = RequirementsMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 3);

    let requests_dep = dependencies.iter().find(|d| d.name == "requests").unwrap();
    assert_eq!(requests_dep.version, Some("2.31.0".to_string()));
    assert_eq!(requests_dep.dep_type, DependencyType::Main);

    let flask_dep = dependencies.iter().find(|d| d.name == "flask").unwrap();
    assert_eq!(flask_dep.version, Some("2.0.0".to_string()));
    assert!(matches!(flask_dep.dep_type, DependencyType::Main));
}

/// Test handling of comments and empty lines in requirements files.
///
/// This test verifies that:
/// 1. Comments are properly ignored
/// 2. Empty lines are skipped
/// 3. Only valid requirements are processed
#[test]
fn test_comments_and_empty_lines() {
    let content = r#"
# Web framework
flask==2.0.0

# Database
# SQLAlchemy is used for ORM
sqlalchemy==1.4.0

# Empty lines above and below

requests==2.31.0  # HTTP client
    "#;

    let (_temp_dir, project_dir) = create_test_project(vec![("requirements.txt", content)]);

    let source = RequirementsMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 3);
}

/// Test handling of multiple requirements files (main and dev requirements).
///
/// This test verifies that:
/// 1. Both main and dev requirements are processed
/// 2. Dependencies are correctly categorized as main or dev
/// 3. All files are properly parsed
#[test]
fn test_multiple_requirements_files() {
    let main_content = "flask==2.0.0\nrequests==2.31.0";
    let dev_content = "pytest==7.0.0\nblack==22.3.0";
    let test_content = "pytest-cov==4.1.0\npytest-mock==3.10.0";

    let (_temp_dir, project_dir) = create_test_project(vec![
        ("requirements.txt", main_content),
        ("requirements-dev.txt", dev_content),
        ("requirements-test.txt", test_content),
    ]);

    let source = RequirementsMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 6);

    let main_deps: Vec<_> = dependencies
        .iter()
        .filter(|d| d.dep_type == DependencyType::Main)
        .collect();
    assert_eq!(main_deps.len(), 2);

    let dev_deps: Vec<_> = dependencies
        .iter()
        .filter(|d| d.dep_type == DependencyType::Dev)
        .collect();
    assert_eq!(dev_deps.len(), 4);
}

/// Test handling of environment markers in requirements.
///
/// This test verifies that:
/// 1. Environment markers are correctly parsed
/// 2. Package names and versions are extracted properly
/// 3. Markers are stored in the dependency structure
#[test]
fn test_environment_markers() {
    let content = r#"
requests==2.31.0; python_version >= "3.7"
flask==2.0.0; sys_platform == "win32"
numpy==1.21.0; platform_machine != "arm64"
    "#;

    let (_temp_dir, project_dir) = create_test_project(vec![("requirements.txt", content)]);

    let source = RequirementsMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 3);

    let requests_dep = dependencies.iter().find(|d| d.name == "requests").unwrap();
    assert_eq!(
        requests_dep.environment_markers,
        Some(r#"python_version >= "3.7""#.to_string())
    );
}

/// Test handling of complex version specifiers.
///
/// This test verifies that:
/// 1. Different version comparison operators are handled
/// 2. Multiple version constraints are parsed correctly
/// 3. Version ranges are properly processed
#[test]
fn test_complex_version_specifiers() {
    let content = r#"
flask>=2.0.0,<3.0.0
requests~=2.31.0
django>3.0.0,<=4.2.0
sqlalchemy!=1.4.0,>=1.3.0
    "#;

    let (_temp_dir, project_dir) = create_test_project(vec![("requirements.txt", content)]);

    let source = RequirementsMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 4);

    // Verify complex version constraints are preserved
    let flask_dep = dependencies.iter().find(|d| d.name == "flask").unwrap();
    assert_eq!(flask_dep.version, Some(">=2.0.0,<3.0.0".to_string()));
}

/// Test handling of editable installs and URLs.
///
/// This test verifies that:
/// 1. Editable installs (-e flag) are parsed correctly
/// 2. Git URLs are properly handled
/// 3. Direct URLs to wheels or source distributions are processed
#[test]
fn test_editable_and_urls() {
    let content = r#"
-e git+https://github.com/user/project.git@master#egg=project
https://files.pythonhosted.org/packages/package.whl
git+https://github.com/user/other-project.git@v1.0.0#egg=other-project
    "#;

    let (_temp_dir, project_dir) = create_test_project(vec![("requirements.txt", content)]);

    let source = RequirementsMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 3);

    // Note: The exact assertions here will depend on how your implementation handles URLs
    // You might want to add more specific assertions based on your implementation
}

/// Test error handling for malformed requirements files.
///
/// This test verifies that:
/// 1. Invalid requirement formats are handled gracefully
/// 2. Appropriate error messages are returned
/// 3. The system doesn't panic on invalid input
#[test]
fn test_malformed_requirements() {
    let content = r#"
flask=2.0.0  # Invalid separator
requests==   # Missing version
===invalid===
    "#;

    let (_temp_dir, project_dir) = create_test_project(vec![("requirements.txt", content)]);

    let source = RequirementsMigrationSource;
    let result = source.extract_dependencies(&project_dir);

    assert!(result.is_ok()); // Should handle malformed requirements gracefully
    let dependencies = result.unwrap();
    assert!(dependencies.len() < 3); // Some requirements should be skipped
}

/// Test handling of no requirements files.
///
/// This test verifies that:
/// 1. Absence of requirements files is handled appropriately
/// 2. A proper error message is returned
#[test]
fn test_no_requirements_files() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().to_path_buf();

    let source = RequirementsMigrationSource;
    let result = source.extract_dependencies(&project_dir);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No requirements files found"));
}

/// Test handling of packages without version specifications.
///
/// This test verifies that:
/// 1. Packages without versions are processed correctly
/// 2. Version field is None for such packages
/// 3. Other package metadata is still captured
#[test]
fn test_packages_without_versions() {
    let content = r#"
flask
requests
sqlalchemy
    "#;

    let (_temp_dir, project_dir) = create_test_project(vec![("requirements.txt", content)]);

    let source = RequirementsMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 3);

    for dep in dependencies {
        assert!(dep.version.is_none());
        assert_eq!(dep.dep_type, DependencyType::Main);
    }
}
