use std::fs;
use tempfile::TempDir;
use uv_migrator::DependencyType;
use uv_migrator::migrators::MigrationSource;
use uv_migrator::migrators::pipenv::PipenvMigrationSource;

/// Test extracting dependencies from a simple Pipenv project
///
/// This test verifies that a basic Pipenv configuration
/// has its dependencies correctly extracted without needing to run the full migration.
/// We manually create a basic Pipfile with minimal content to test the parser.
#[test]
fn test_extract_pipenv_dependencies() {
    // Create a temporary directory for our test project
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().to_path_buf();

    // Create a simple Pipfile
    let pipfile_content = r#"[[source]]
url = "https://pypi.org/simple"
verify_ssl = true
name = "pypi"

[packages]
fastapi = "*"

[dev-packages]

[requires]
python_version = "3.12"
"#;

    // Create Pipfile.lock to ensure detection works
    let pipfile_lock_content = r#"{
    "default": {
        "fastapi": {
            "version": "==0.115.8"
        }
    },
    "develop": {}
}"#;

    // Write the Pipfile to the temporary directory
    fs::write(project_dir.join("Pipfile"), pipfile_content).unwrap();
    fs::write(project_dir.join("Pipfile.lock"), pipfile_lock_content).unwrap();

    // Use PipenvMigrationSource to extract dependencies
    let source = PipenvMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    // Verify the dependencies were extracted correctly
    assert!(!dependencies.is_empty(), "No dependencies were extracted");

    // Check if fastapi is in the dependencies
    let fastapi_dep = dependencies.iter().find(|d| d.name == "fastapi");
    assert!(fastapi_dep.is_some(), "FastAPI dependency not found");

    // Check if it has the correct type
    let fastapi_dep = fastapi_dep.unwrap();
    assert_eq!(fastapi_dep.dep_type, DependencyType::Main);

    // Check if python_version is correctly ignored
    let python_dep = dependencies.iter().find(|d| d.name == "python_version");
    assert!(python_dep.is_none(), "Python version should be ignored");

    // Verify there are no dev dependencies
    let dev_deps: Vec<_> = dependencies
        .iter()
        .filter(|d| matches!(d.dep_type, DependencyType::Dev))
        .collect();
    assert!(dev_deps.is_empty(), "There should be no dev dependencies");
}

/// Test Pipenv project structure detection
///
/// This test verifies that a directory with both Pipfile and Pipfile.lock
/// is correctly detected as a Pipenv project.
#[test]
fn test_detect_pipenv_project() {
    // Create a temporary directory for our test project
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().to_path_buf();

    // Create both Pipfile and Pipfile.lock
    fs::write(project_dir.join("Pipfile"), "").unwrap();
    fs::write(project_dir.join("Pipfile.lock"), "{}").unwrap();

    // Use PipenvMigrationSource to detect project type
    let is_pipenv = PipenvMigrationSource::detect_project_type(&project_dir);

    // Verify it's detected as a Pipenv project
    assert!(
        is_pipenv,
        "Directory with Pipfile and Pipfile.lock should be detected as Pipenv project"
    );

    // Create another temporary directory with only one file
    let temp_dir2 = TempDir::new().unwrap();
    let project_dir2 = temp_dir2.path().to_path_buf();

    // Add only Pipfile
    fs::write(project_dir2.join("Pipfile"), "").unwrap();

    // Verify it's not detected as a Pipenv project
    let is_pipenv2 = PipenvMigrationSource::detect_project_type(&project_dir2);
    assert!(
        !is_pipenv2,
        "Directory with only Pipfile should not be detected as Pipenv project"
    );

    // Test with only Pipfile.lock
    let temp_dir3 = TempDir::new().unwrap();
    let project_dir3 = temp_dir3.path().to_path_buf();

    // Add only Pipfile.lock
    fs::write(project_dir3.join("Pipfile.lock"), "{}").unwrap();

    // Verify it's not detected as a Pipenv project
    let is_pipenv3 = PipenvMigrationSource::detect_project_type(&project_dir3);
    assert!(
        !is_pipenv3,
        "Directory with only Pipfile.lock should not be detected as Pipenv project"
    );
}
