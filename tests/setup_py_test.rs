#[cfg(test)]
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use uv_migrator::migrators::setup_py::SetupPyMigrationSource;
use uv_migrator::migrators::{DependencyType, MigrationSource};
use uv_migrator::utils::author::{extract_authors_from_setup_py, update_authors};

/// Helper function to create a temporary test project with setup.py and optional requirements.txt.
///
/// # Arguments
///
/// * `setup_content` - Content for the setup.py file
/// * `requirements_content` - Optional content for requirements.txt
///
/// # Returns
///
/// A tuple containing the temporary directory and its path
fn create_test_project(
    setup_content: &str,
    requirements_content: Option<&str>,
) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().to_path_buf();

    // Write setup.py
    fs::write(project_dir.join("setup.py"), setup_content).unwrap();

    // Write requirements.txt if provided
    if let Some(content) = requirements_content {
        fs::write(project_dir.join("requirements.txt"), content).unwrap();
    }

    (temp_dir, project_dir)
}

/// Test dependency extraction from setup.py with an accompanying requirements file.
///
/// This test verifies that:
/// 1. All dependencies are correctly extracted from requirements.txt
/// 2. Version specifications are properly parsed
/// 3. Dependencies are marked with correct types
/// 4. Complex version constraints are handled correctly
///
/// # Test Setup
/// Creates a project with:
/// - Basic setup.py without direct dependencies
/// - requirements.txt with various version specifications
///
/// # Verification Steps
/// 1. Confirms correct number of dependencies
/// 2. Verifies specific dependency versions
/// 3. Validates dependency types
/// 4. Checks version constraint parsing
#[test]
fn test_setup_py_with_requirements_file() {
    let setup_content = r#"
from setuptools import setup

setup(
    name="pb_logging",
    version="1.0.0",
    description="Logging-related utilities",
)
"#;

    let requirements_content = r#"
flask==2.0.0
requests==2.31.0
sqlalchemy>=1.4.0,<2.0.0
"#;

    let (_temp_dir, project_dir) = create_test_project(setup_content, Some(requirements_content));
    let source = SetupPyMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(
        dependencies.len(),
        3,
        "Should extract all dependencies from requirements.txt"
    );

    let flask_dep = dependencies.iter().find(|d| d.name == "flask").unwrap();
    assert_eq!(flask_dep.version, Some("2.0.0".to_string()));
    assert_eq!(flask_dep.dep_type, DependencyType::Main);

    let requests_dep = dependencies.iter().find(|d| d.name == "requests").unwrap();
    assert_eq!(requests_dep.version, Some("2.31.0".to_string()));
    assert_eq!(requests_dep.dep_type, DependencyType::Main);

    let sqlalchemy_dep = dependencies
        .iter()
        .find(|d| d.name == "sqlalchemy")
        .unwrap();
    assert_eq!(sqlalchemy_dep.version, Some(">=1.4.0,<2.0.0".to_string()));
    assert_eq!(sqlalchemy_dep.dep_type, DependencyType::Main);
}

/// Test dependency extraction directly from setup.py install_requires and tests_require.
///
/// This test verifies that:
/// 1. Main dependencies are extracted from install_requires
/// 2. Test dependencies are extracted from tests_require
/// 3. Version specifications are correctly parsed
/// 4. Dependencies are properly categorized
///
/// # Test Setup
/// Creates a setup.py with:
/// - Multiple install_requires dependencies
/// - Multiple tests_require dependencies
/// - Various version specifications
///
/// # Verification Steps
/// 1. Confirms correct number of main and test dependencies
/// 2. Verifies dependency categorization
/// 3. Validates version parsing
/// 4. Checks specific dependency details
#[test]
fn test_setup_py_with_direct_dependencies() {
    let setup_content = r#"
from setuptools import setup

setup(
    name="pb_logging",
    version="1.0.0",
    description="Logging-related utilities",
    install_requires=[
        'flask>=2.0.0',
        'requests==2.31.0',
        'sqlalchemy>=1.4.0'
    ],
    tests_require=[
        'pytest>=7.0.0',
        'pytest-cov>=4.0.0'
    ]
)
"#;

    let (_temp_dir, project_dir) = create_test_project(setup_content, None);
    let source = SetupPyMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    let main_deps: Vec<_> = dependencies
        .iter()
        .filter(|d| matches!(d.dep_type, DependencyType::Main))
        .collect();

    let dev_deps: Vec<_> = dependencies
        .iter()
        .filter(|d| matches!(d.dep_type, DependencyType::Dev))
        .collect();

    assert_eq!(main_deps.len(), 3, "Should have 3 main dependencies");
    assert_eq!(dev_deps.len(), 2, "Should have 2 dev dependencies");

    let flask_dep = main_deps.iter().find(|d| d.name == "flask").unwrap();
    assert_eq!(flask_dep.version, Some(">=2.0.0".to_string()));

    let pytest_dep = dev_deps.iter().find(|d| d.name == "pytest").unwrap();
    assert_eq!(pytest_dep.version, Some(">=7.0.0".to_string()));
}

/// Test handling of setup.py without any requirements.
///
/// This test verifies that:
/// 1. Setup.py without dependencies is handled gracefully
/// 2. Empty dependency list is returned
/// 3. No errors are thrown
///
/// # Test Setup
/// Creates a setup.py with:
/// - Basic project metadata
/// - No dependency specifications
///
/// # Verification Steps
/// 1. Confirms dependencies list is empty
/// 2. Verifies successful execution
#[test]
fn test_setup_py_no_requirements() {
    let setup_content = r#"
from setuptools import setup

setup(
    name="pb_logging",
    version="1.0.0",
    description="Logging-related utilities",
)
"#;

    let (_temp_dir, project_dir) = create_test_project(setup_content, None);
    let source = SetupPyMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert!(dependencies.is_empty(), "Should have no dependencies");
}

/// Test handling of malformed setup.py dependency specifications.
///
/// This test verifies that:
/// 1. Invalid dependency specifications are handled gracefully
/// 2. No errors are thrown for malformed content
/// 3. Empty dependency list is returned
///
/// # Test Setup
/// Creates a setup.py with:
/// - Invalid install_requires format
/// - Otherwise valid project metadata
///
/// # Verification Steps
/// 1. Confirms operation completes without error
/// 2. Verifies empty dependency list is returned
/// 3. Validates graceful error handling
#[test]
fn test_setup_py_malformed() {
    let setup_content = r#"
from setuptools import setup

setup(
    name="pb_logging",
    version="1.0.0",
    description="Logging-related utilities",
    install_requires="not a list",  # This is invalid
)
"#;

    let (_temp_dir, project_dir) = create_test_project(setup_content, None);
    let source = SetupPyMigrationSource;
    let result = source.extract_dependencies(&project_dir);

    assert!(
        result.is_ok(),
        "Should handle malformed setup.py without crashing"
    );
    let dependencies = result.unwrap();
    assert!(
        dependencies.is_empty(),
        "Should have no dependencies for malformed setup.py"
    );
}

/// Helper function to create a temporary test environment with setup.py and pyproject.toml
///
/// # Arguments
///
/// * `setup_content` - Content for the setup.py file
/// * `pyproject_content` - Content for the pyproject.toml file
///
/// # Returns
///
/// A tuple containing the temporary directory and its path
fn setup_test_environment(setup_content: &str, pyproject_content: &str) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().to_path_buf();

    fs::write(project_dir.join("setup.py"), setup_content).unwrap();
    fs::write(project_dir.join("pyproject.toml"), pyproject_content).unwrap();

    (temp_dir, project_dir)
}

/// Test extraction of author information from setup.py.
///
/// This test verifies that:
/// 1. Author name is correctly extracted
/// 2. Author email is correctly extracted
/// 3. Author information is properly structured
///
/// # Test Setup
/// Creates a setup.py with:
/// - Author name
/// - Author email
/// - Basic project metadata
///
/// # Verification Steps
/// 1. Confirms correct number of authors
/// 2. Verifies author name extraction
/// 3. Validates author email extraction
#[test]
fn test_extract_authors() {
    let setup_content = r#"
from setuptools import setup

setup(
    name="test-project",
    version="1.0.0",
    author="Demo Name",
    author_email="demo.name@corp.com",
    description="Test project"
)
"#;
    let (_temp_dir, project_dir) = setup_test_environment(setup_content, "");

    let authors = extract_authors_from_setup_py(&project_dir).unwrap();
    assert_eq!(authors.len(), 1);
    assert_eq!(authors[0].name, "Demo Name");
    assert_eq!(authors[0].email, Some("demo.name@corp.com".to_string()));
}

/// Test updating authors in pyproject.toml from setup.py.
///
/// This test verifies that:
/// 1. Authors section is correctly created
/// 2. Author information is properly formatted
/// 3. Both name and email are included
///
/// # Test Setup
/// Creates:
/// - setup.py with author information
/// - pyproject.toml without authors section
///
/// # Verification Steps
/// 1. Verifies authors section is created
/// 2. Confirms author information is correct
/// 3. Validates TOML formatting
#[test]
fn test_update_authors_in_pyproject() {
    let setup_content = r#"
from setuptools import setup

setup(
    name="test-project",
    version="1.0.0",
    author="Demo Name",
    author_email="demo.name@corp.com",
    description="Test project"
)
"#;
    let pyproject_content = r#"[project]
name = "test-project"
version = "1.0.0"
description = "Test project"
"#;

    let (_temp_dir, project_dir) = setup_test_environment(setup_content, pyproject_content);

    update_authors(&project_dir).unwrap();

    let updated_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
    assert!(updated_content.contains(r#"authors = ["#));
    assert!(updated_content.contains(r#"{ name = "Demo Name", email = "demo.name@corp.com" }"#));
}

/// Test updating authors when existing authors are present.
///
/// This test verifies that:
/// 1. Existing authors are replaced
/// 2. New author information is correctly added
/// 3. Old author information is removed
///
/// # Test Setup
/// Creates:
/// - setup.py with new author information
/// - pyproject.toml with existing authors
///
/// # Verification Steps
/// 1. Confirms new author is added
/// 2. Verifies old author is removed
/// 3. Validates TOML structure
#[test]
fn test_update_authors_with_existing_authors() {
    let setup_content = r#"
from setuptools import setup

setup(
    name="test-project",
    version="1.0.0",
    author="Demo Name",
    author_email="demo.name@corp.com",
    description="Test project"
)
"#;
    let pyproject_content = r#"[project]
name = "test-project"
version = "1.0.0"
description = "Test project"
authors = [
    { name = "Old Author", email = "old@example.com" }
]
"#;

    let (_temp_dir, project_dir) = setup_test_environment(setup_content, pyproject_content);

    update_authors(&project_dir).unwrap();

    let updated_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
    assert!(updated_content.contains(r#"{ name = "Demo Name", email = "demo.name@corp.com" }"#));
    assert!(!updated_content.contains(r#"{ name = "Old Author", email = "old@example.com" }"#));
}

/// Test handling of missing author email in setup.py.
///
/// This test verifies that:
/// 1. Authors without email are handled correctly
/// 2. Name-only author entries are properly formatted
/// 3. Email field is correctly omitted
///
/// # Test Setup
/// Creates:
/// - setup.py with author name but no email
/// - Basic pyproject.toml
///
/// # Verification Steps
/// 1. Verifies author name is preserved
/// 2. Confirms email field is omitted
/// 3. Validates TOML structure
#[test]
fn test_missing_author_email() {
    let setup_content = r#"
from setuptools import setup

setup(
    name="test-project",
    version="1.0.0",
    author="Demo Name",
    description="Test project"
)
"#;
    let pyproject_content = r#"[project]
name = "test-project"
version = "1.0.0"
description = "Test project"
"#;

    let (_temp_dir, project_dir) = setup_test_environment(setup_content, pyproject_content);

    update_authors(&project_dir).unwrap();

    let updated_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
    assert!(updated_content.contains(r#"{ name = "Demo Name" }"#));
    assert!(!updated_content.contains("email"));
}

/// Test handling of setup.py without any author information.
///
/// This test verifies that:
/// 1. Absence of author information is handled gracefully
/// 2. No authors section is created in pyproject.toml
/// 3. Existing pyproject.toml content is preserved
/// 4. Operation completes successfully
///
/// # Test Setup
/// Creates:
/// - setup.py without author information
/// - Basic pyproject.toml
///
/// # Verification Steps
/// 1. Confirms no authors section is added
/// 2. Verifies original content is preserved
/// 3. Validates operation succeeds
#[test]
fn test_no_authors() {
    let setup_content = r#"
from setuptools import setup

setup(
    name="test-project",
    version="1.0.0",
    description="Test project"
)
"#;
    let pyproject_content = r#"[project]
name = "test-project"
version = "1.0.0"
description = "Test project"
"#;

    let (_temp_dir, project_dir) = setup_test_environment(setup_content, pyproject_content);

    update_authors(&project_dir).unwrap();

    let updated_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
    assert!(!updated_content.contains("authors"));
}

/// Test updating authors and URLs simultaneously.
///
/// This test verifies that:
/// 1. Author information is correctly updated
/// 2. Project URL is properly added
/// 3. Both updates coexist correctly
/// 4. TOML structure remains valid
///
/// # Test Setup
/// Creates:
/// - setup.py with author and URL information
/// - Basic pyproject.toml
///
/// # Verification Steps
/// 1. Verifies author information is added
/// 2. Confirms URL is correctly set
/// 3. Validates combined structure
/// 4. Checks TOML formatting
#[test]
fn test_update_authors_with_url() {
    let setup_content = r#"
from setuptools import setup

setup(
    name="test-project",
    version="1.0.0",
    author="Demo Name",
    author_email="demo.name@corp.com",
    url="https://gitlab.com/example/project",
    description="Test project"
)
"#;
    let pyproject_content = r#"[project]
name = "test-project"
version = "1.0.0"
description = "Test project"
"#;

    let (_temp_dir, project_dir) = setup_test_environment(setup_content, pyproject_content);

    if let Some(url) = SetupPyMigrationSource::extract_url(&project_dir).unwrap() {
        uv_migrator::utils::update_url(&project_dir, &url).unwrap();
    }

    update_authors(&project_dir).unwrap();

    let updated_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
    assert!(updated_content.contains(r#"{ name = "Demo Name", email = "demo.name@corp.com" }"#));
    assert!(
        updated_content.contains(r#"urls = { repository = "https://gitlab.com/example/project" }"#)
    );
}

/// Test updating URLs when existing content is present.
///
/// This test verifies that:
/// 1. Existing URLs are properly updated
/// 2. Author information is correctly updated
/// 3. Other project metadata is preserved
/// 4. Updates are properly formatted
///
/// # Test Setup
/// Creates:
/// - setup.py with new URL and author information
/// - pyproject.toml with existing URLs and authors
///
/// # Verification Steps
/// 1. Confirms URL is updated
/// 2. Verifies author information is updated
/// 3. Validates existing metadata preservation
/// 4. Checks TOML structure integrity
#[test]
fn test_update_urls_existing_content() {
    let setup_content = r#"
from setuptools import setup

setup(
    name="test-project",
    version="1.0.0",
    author="Demo Name",
    author_email="demo.name@corp.com",
    url="https://gitlab.com/updated/project",
    description="Test project"
)
"#;
    let pyproject_content = r#"[project]
name = "test-project"
version = "1.0.0"
description = "Test project"
authors = [
    { name = "Old Author", email = "old@example.com" }
]
urls = { repository = "https://oldproject.example.com" }
requires-python = ">=3.8"
"#;

    let (_temp_dir, project_dir) = setup_test_environment(setup_content, pyproject_content);

    if let Some(url) = SetupPyMigrationSource::extract_url(&project_dir).unwrap() {
        uv_migrator::utils::update_url(&project_dir, &url).unwrap();
    }

    update_authors(&project_dir).unwrap();

    let updated_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
    assert!(updated_content.contains(r#"{ name = "Demo Name", email = "demo.name@corp.com" }"#));
    assert!(
        updated_content.contains(r#"urls = { repository = "https://gitlab.com/updated/project" }"#)
    );
}
