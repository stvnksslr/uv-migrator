use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use uv_migrator::DependencyType;
use uv_migrator::migrators::MigrationSource;
use uv_migrator::migrators::requirements::RequirementsMigrationSource;
use uv_migrator::migrators::{self};

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
    assert_eq!(flask_dep.version, Some(">=2.0.0".to_string()));
    assert!(matches!(flask_dep.dep_type, DependencyType::Main));

    let sqlalchemy_dep = dependencies
        .iter()
        .find(|d| d.name == "sqlalchemy")
        .unwrap();
    assert_eq!(sqlalchemy_dep.version, Some("<2.0.0".to_string()));
    assert!(matches!(sqlalchemy_dep.dep_type, DependencyType::Main));
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

/// Test handling of multiple requirements files (main, dev, and test requirements).
///
/// This test verifies that:
/// 1. Requirements from all files are processed
/// 2. Dependencies are correctly categorized into main, dev, and test groups
/// 3. The correct number of dependencies are found in each category
/// 4. Specific packages are found in their expected categories
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

    // Verify total number of dependencies
    assert_eq!(
        dependencies.len(),
        6,
        "Total number of dependencies should be 6"
    );

    // Check main dependencies
    let main_deps: Vec<_> = dependencies
        .iter()
        .filter(|d| matches!(d.dep_type, DependencyType::Main))
        .collect();
    assert_eq!(main_deps.len(), 2, "Should have 2 main dependencies");
    assert!(
        main_deps.iter().any(|d| d.name == "flask"),
        "Main dependencies should include flask"
    );
    assert!(
        main_deps.iter().any(|d| d.name == "requests"),
        "Main dependencies should include requests"
    );

    // Check dev dependencies
    let dev_deps: Vec<_> = dependencies
        .iter()
        .filter(|d| matches!(d.dep_type, DependencyType::Dev))
        .collect();
    assert_eq!(dev_deps.len(), 2, "Should have 2 dev dependencies");
    assert!(
        dev_deps.iter().any(|d| d.name == "pytest"),
        "Dev dependencies should include pytest"
    );
    assert!(
        dev_deps.iter().any(|d| d.name == "black"),
        "Dev dependencies should include black"
    );

    // Check test dependencies
    let test_deps: Vec<_> = dependencies
        .iter()
        .filter(|d| matches!(d.dep_type, DependencyType::Group(ref g) if g == "test"))
        .collect();
    assert_eq!(test_deps.len(), 2, "Should have 2 test dependencies");
    assert!(
        test_deps.iter().any(|d| d.name == "pytest-cov"),
        "Test dependencies should include pytest-cov"
    );
    assert!(
        test_deps.iter().any(|d| d.name == "pytest-mock"),
        "Test dependencies should include pytest-mock"
    );

    // Verify versions for a sample of dependencies
    if let Some(flask_dep) = dependencies.iter().find(|d| d.name == "flask") {
        assert_eq!(
            flask_dep.version,
            Some("2.0.0".to_string()),
            "Flask version should be 2.0.0"
        );
    }
    if let Some(pytest_dep) = dependencies.iter().find(|d| d.name == "pytest") {
        assert_eq!(
            pytest_dep.version,
            Some("7.0.0".to_string()),
            "Pytest version should be 7.0.0"
        );
    }
    if let Some(pytest_cov_dep) = dependencies.iter().find(|d| d.name == "pytest-cov") {
        assert_eq!(
            pytest_cov_dep.version,
            Some("4.1.0".to_string()),
            "Pytest-cov version should be 4.1.0"
        );
    }
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
django-filters~=23.5
boto3~=1.35
    "#;

    let (_temp_dir, project_dir) = create_test_project(vec![("requirements.txt", content)]);

    let source = RequirementsMigrationSource;
    let dependencies = source.extract_dependencies(&project_dir).unwrap();

    assert_eq!(dependencies.len(), 6);

    // Verify complex version constraints are preserved
    let flask_dep = dependencies.iter().find(|d| d.name == "flask").unwrap();
    assert_eq!(flask_dep.version, Some(">=2.0.0,<3.0.0".to_string()));

    // Verify tilde-equal is preserved
    let requests_dep = dependencies.iter().find(|d| d.name == "requests").unwrap();
    assert_eq!(requests_dep.version, Some("~=2.31.0".to_string()));

    // Verify multiple constraints with inequality
    let django_dep = dependencies.iter().find(|d| d.name == "django").unwrap();
    assert_eq!(django_dep.version, Some(">3.0.0,<=4.2.0".to_string()));

    // Verify complex constraints with not-equal
    let sqlalchemy_dep = dependencies
        .iter()
        .find(|d| d.name == "sqlalchemy")
        .unwrap();
    assert_eq!(sqlalchemy_dep.version, Some("!=1.4.0,>=1.3.0".to_string()));

    // Verify tilde-equal cases
    let filters_dep = dependencies
        .iter()
        .find(|d| d.name == "django-filters")
        .unwrap();
    assert_eq!(filters_dep.version, Some("~=23.5".to_string()));

    let boto3_dep = dependencies.iter().find(|d| d.name == "boto3").unwrap();
    assert_eq!(boto3_dep.version, Some("~=1.35".to_string()));
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
}

/// Test handling of malformed requirements files.
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

#[cfg(test)]
/// Tests for the dependency group merging functionality
///
/// This module tests the behavior of merging different dependency groups into the dev group.
/// It verifies that group dependencies are correctly identified, merged, and that all
/// dependency metadata is preserved during the merge process.
mod merge_groups_tests {
    use super::*;

    /// Helper function to set up test requirements files.
    ///
    /// Creates a temporary directory and populates it with requirements files based on
    /// the provided content. Always creates requirements.txt and optionally creates
    /// requirements-dev.txt and requirements-test.txt if content is provided.
    ///
    /// # Arguments
    ///
    /// * `main_content` - Content for the main requirements.txt file
    /// * `dev_content` - Optional content for requirements-dev.txt
    /// * `test_content` - Optional content for requirements-test.txt
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// * The temporary directory handle (cleaned up when dropped)
    /// * Path to the project directory containing the requirements files
    fn setup_test_files(
        main_content: &str,
        dev_content: Option<&str>,
        test_content: Option<&str>,
    ) -> (TempDir, std::path::PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // Create main requirements.txt
        std::fs::write(project_dir.join("requirements.txt"), main_content).unwrap();

        // Create requirements-dev.txt if content provided
        if let Some(content) = dev_content {
            std::fs::write(project_dir.join("requirements-dev.txt"), content).unwrap();
        }

        // Create requirements-test.txt if content provided
        if let Some(content) = test_content {
            std::fs::write(project_dir.join("requirements-test.txt"), content).unwrap();
        }

        (temp_dir, project_dir)
    }

    /// Test merging of dependency groups in requirements files.
    ///
    /// This test verifies that:
    /// 1. Dependencies from different requirements files are correctly identified with their types
    /// 2. The merge process correctly converts all group dependencies to dev dependencies
    /// 3. Main dependencies remain unchanged during the merge
    /// 4. The final merged dependencies contain the expected number of dev dependencies
    /// 5. No group dependencies remain after the merge operation
    ///
    /// # Test Setup
    /// * Creates a main requirements.txt with two packages
    /// * Creates a requirements-dev.txt with two packages
    /// * Creates a requirements-test.txt with two packages
    ///
    /// # Verification Steps
    /// 1. Verifies initial dependency counts by type
    /// 2. Applies group merging
    /// 3. Verifies final dependency counts and types
    /// 4. Ensures no group dependencies remain
    #[test]
    fn test_merge_groups_requirements() {
        let main_content = "flask==2.0.0\nrequests==2.31.0";
        let dev_content = "pytest==7.0.0\nblack==22.3.0";
        let test_content = "pytest-cov==4.1.0\npytest-mock==3.10.0";

        let (_temp_dir, project_dir) =
            setup_test_files(main_content, Some(dev_content), Some(test_content));

        // Extract dependencies normally first
        let source = RequirementsMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        // Verify initial state
        assert_eq!(
            dependencies
                .iter()
                .filter(|d| matches!(d.dep_type, DependencyType::Main))
                .count(),
            2,
            "Should have 2 main dependencies"
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
            4,
            "Should have 4 dev dependencies after merge (original dev + merged test)"
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
}
