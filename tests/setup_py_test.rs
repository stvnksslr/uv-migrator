#[cfg(test)]
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use uv_migrator::DependencyType;
use uv_migrator::migrators::MigrationSource;
use uv_migrator::migrators::setup_py::SetupPyMigrationSource;
use uv_migrator::utils::author::extract_authors_from_setup_py;
use uv_migrator::utils::toml::{read_toml, update_section, write_toml};

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

fn setup_test_environment(setup_content: &str, pyproject_content: &str) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().to_path_buf();

    fs::write(project_dir.join("setup.py"), setup_content).unwrap();
    fs::write(project_dir.join("pyproject.toml"), pyproject_content).unwrap();

    (temp_dir, project_dir)
}

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

    // First extract the authors
    let authors = extract_authors_from_setup_py(&project_dir).unwrap();

    // Then update the pyproject.toml with the extracted authors
    let mut doc = read_toml(&project_dir.join("pyproject.toml")).unwrap();
    let mut authors_array = toml_edit::Array::new();
    for author in &authors {
        let mut table = toml_edit::InlineTable::new();
        table.insert(
            "name",
            toml_edit::Value::String(toml_edit::Formatted::new(author.name.clone())),
        );
        if let Some(ref email) = author.email {
            table.insert(
                "email",
                toml_edit::Value::String(toml_edit::Formatted::new(email.clone())),
            );
        }
        authors_array.push(toml_edit::Value::InlineTable(table));
    }
    update_section(
        &mut doc,
        &["project", "authors"],
        toml_edit::Item::Value(toml_edit::Value::Array(authors_array)),
    );
    write_toml(&project_dir.join("pyproject.toml"), &mut doc).unwrap();

    let updated_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
    assert!(updated_content.contains("authors = ["));
    assert!(updated_content.contains(r#"{ name = "Demo Name", email = "demo.name@corp.com" }"#));
}

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

    let authors = extract_authors_from_setup_py(&project_dir).unwrap();

    let mut doc = read_toml(&project_dir.join("pyproject.toml")).unwrap();
    let mut authors_array = toml_edit::Array::new();
    for author in &authors {
        let mut table = toml_edit::InlineTable::new();
        table.insert(
            "name",
            toml_edit::Value::String(toml_edit::Formatted::new(author.name.clone())),
        );
        if let Some(ref email) = author.email {
            table.insert(
                "email",
                toml_edit::Value::String(toml_edit::Formatted::new(email.clone())),
            );
        }
        authors_array.push(toml_edit::Value::InlineTable(table));
    }
    update_section(
        &mut doc,
        &["project", "authors"],
        toml_edit::Item::Value(toml_edit::Value::Array(authors_array)),
    );
    write_toml(&project_dir.join("pyproject.toml"), &mut doc).unwrap();

    let updated_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
    assert!(updated_content.contains(r#"{ name = "Demo Name", email = "demo.name@corp.com" }"#));
    assert!(!updated_content.contains(r#"{ name = "Old Author", email = "old@example.com" }"#));
}

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

    let authors = extract_authors_from_setup_py(&project_dir).unwrap();

    let mut doc = read_toml(&project_dir.join("pyproject.toml")).unwrap();
    let mut authors_array = toml_edit::Array::new();
    for author in &authors {
        let mut table = toml_edit::InlineTable::new();
        table.insert(
            "name",
            toml_edit::Value::String(toml_edit::Formatted::new(author.name.clone())),
        );
        if let Some(ref email) = author.email {
            table.insert(
                "email",
                toml_edit::Value::String(toml_edit::Formatted::new(email.clone())),
            );
        }
        authors_array.push(toml_edit::Value::InlineTable(table));
    }
    update_section(
        &mut doc,
        &["project", "authors"],
        toml_edit::Item::Value(toml_edit::Value::Array(authors_array)),
    );
    write_toml(&project_dir.join("pyproject.toml"), &mut doc).unwrap();

    let updated_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
    assert!(updated_content.contains(r#"{ name = "Demo Name" }"#));
    assert!(!updated_content.contains("email"));
}

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

    // Update URL
    if let Some(url) = SetupPyMigrationSource::extract_url(&project_dir).unwrap() {
        uv_migrator::utils::update_url(&project_dir, &url).unwrap();
    }

    // Update authors
    let authors = extract_authors_from_setup_py(&project_dir).unwrap();

    let mut doc = read_toml(&project_dir.join("pyproject.toml")).unwrap();
    let mut authors_array = toml_edit::Array::new();
    for author in &authors {
        let mut table = toml_edit::InlineTable::new();
        table.insert(
            "name",
            toml_edit::Value::String(toml_edit::Formatted::new(author.name.clone())),
        );
        if let Some(ref email) = author.email {
            table.insert(
                "email",
                toml_edit::Value::String(toml_edit::Formatted::new(email.clone())),
            );
        }
        authors_array.push(toml_edit::Value::InlineTable(table));
    }
    update_section(
        &mut doc,
        &["project", "authors"],
        toml_edit::Item::Value(toml_edit::Value::Array(authors_array)),
    );
    write_toml(&project_dir.join("pyproject.toml"), &mut doc).unwrap();

    let updated_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
    assert!(updated_content.contains(r#"{ name = "Demo Name", email = "demo.name@corp.com" }"#));
    assert!(
        updated_content.contains(r#"urls = { repository = "https://gitlab.com/example/project" }"#)
    );
}

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

    // Update URL
    if let Some(url) = SetupPyMigrationSource::extract_url(&project_dir).unwrap() {
        uv_migrator::utils::update_url(&project_dir, &url).unwrap();
    }

    // Update authors
    let authors = extract_authors_from_setup_py(&project_dir).unwrap();

    let mut doc = read_toml(&project_dir.join("pyproject.toml")).unwrap();
    let mut authors_array = toml_edit::Array::new();
    for author in &authors {
        let mut table = toml_edit::InlineTable::new();
        table.insert(
            "name",
            toml_edit::Value::String(toml_edit::Formatted::new(author.name.clone())),
        );
        if let Some(ref email) = author.email {
            table.insert(
                "email",
                toml_edit::Value::String(toml_edit::Formatted::new(email.clone())),
            );
        }
        authors_array.push(toml_edit::Value::InlineTable(table));
    }
    update_section(
        &mut doc,
        &["project", "authors"],
        toml_edit::Item::Value(toml_edit::Value::Array(authors_array)),
    );
    write_toml(&project_dir.join("pyproject.toml"), &mut doc).unwrap();

    let updated_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
    assert!(updated_content.contains(r#"{ name = "Demo Name", email = "demo.name@corp.com" }"#));
    assert!(
        updated_content.contains(r#"urls = { repository = "https://gitlab.com/updated/project" }"#)
    );
}
