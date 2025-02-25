use log::debug;
use std::path::Path;
use toml_edit::{DocumentMut, Item, Table, Value};

/// Updates the build system configuration in pyproject.toml.
/// This function follows PEP 621 guidelines to determine if a project is a package
/// that needs a build system or an application that can use the default.
///
/// # Arguments
///
/// * `doc` - The TOML document to update
/// * `project_dir` - The project directory path
///
/// # Returns
///
/// * `bool` - Whether any changes were made to the document
pub fn update_build_system(doc: &mut DocumentMut, project_dir: &Path) -> Result<bool, String> {
    debug!("Checking if project needs a build system configuration");
    let old_pyproject_path = project_dir.join("old.pyproject.toml");
    if !old_pyproject_path.exists() {
        return Ok(false);
    }

    // Read old pyproject.toml to analyze the project structure
    let old_content = std::fs::read_to_string(&old_pyproject_path)
        .map_err(|e| format!("Failed to read old.pyproject.toml: {}", e))?;

    let old_doc = old_content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse old.pyproject.toml: {}", e))?;

    // Check if this is a package project according to PEP 621 and Poetry standards
    let is_package_project = determine_if_package_project(&old_doc, project_dir);

    // If it's not a package project, don't add a build-system section
    if !is_package_project {
        debug!("Project appears to be an application, not setting build system");
        return Ok(false);
    }

    debug!("Project appears to be a package, configuring build system with Hatchling");

    // Create new build-system table
    let mut build_system = Table::new();

    // Add requires array
    let mut requires = toml_edit::Array::new();
    requires.push(Value::String(toml_edit::Formatted::new(
        "hatchling".to_string(),
    )));
    build_system.insert("requires", Item::Value(Value::Array(requires)));

    // Add build-backend string
    build_system.insert(
        "build-backend",
        Item::Value(Value::String(toml_edit::Formatted::new(
            "hatchling.build".to_string(),
        ))),
    );

    // Update the document
    doc.insert("build-system", Item::Table(build_system));

    Ok(true)
}

/// Determines if a project is a package (vs an application) based on various indicators
fn determine_if_package_project(doc: &DocumentMut, project_dir: &Path) -> bool {
    // Check for various indicators that this is a package project:

    // 1. Check for Poetry packages configuration with src directory
    let has_poetry_package_config = doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|poetry| {
            // First try `packages` key directly in poetry
            if let Some(packages) = poetry.get("packages").and_then(|p| p.as_array()) {
                Some(packages.iter().any(|pkg| {
                    if let Some(table) = pkg.as_inline_table() {
                        (table.get("from").and_then(|f| f.as_str()) == Some("src"))
                            || (table.get("include").and_then(|i| i.as_str()).is_some()
                                && table.get("from").and_then(|f| f.as_str()) == Some("src"))
                    } else {
                        false
                    }
                }))
            }
            // Try packages array within tool.poetry.packages section
            else if let Some(packages_section) = poetry.get("packages") {
                if let Some(packages_array) =
                    packages_section.get("packages").and_then(|p| p.as_array())
                {
                    Some(packages_array.iter().any(|pkg| {
                        if let Some(table) = pkg.as_inline_table() {
                            (table.get("from").and_then(|f| f.as_str()) == Some("src"))
                                || (table.get("include").and_then(|i| i.as_str()).is_some()
                                    && table.get("from").and_then(|f| f.as_str()) == Some("src"))
                        } else {
                            false
                        }
                    }))
                } else {
                    // Just having a packages section is a strong indication it's a package
                    Some(true)
                }
            } else {
                None
            }
        })
        .unwrap_or(false);

    // OR: Check for Poetry packages section in any configuration
    let has_poetry_packages = has_poetry_package_config
        || doc
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|poetry| poetry.get("packages"))
            .is_some();

    // Also check for packages configuration in Poetry 2.0 format
    let has_poetry2_packages = doc
        .get("project")
        .and_then(|project| project.get("packages"))
        .is_some();

    if has_poetry_packages || has_poetry2_packages {
        debug!("Project has Poetry package configuration");
        return true;
    }

    // 2. Check for setup.py or setup.cfg which would indicate a package
    if project_dir.join("setup.py").exists() || project_dir.join("setup.cfg").exists() {
        debug!("Project has setup.py or setup.cfg");
        return true;
    }

    // 3. Check for typical package structure (src directory with an __init__.py file)
    let src_dir = project_dir.join("src");
    if src_dir.exists() && src_dir.is_dir() {
        // Check if there are any __init__.py files in the src directory
        if std::fs::read_dir(&src_dir).ok().is_some_and(|entries| {
            entries
                .flatten()
                .any(|entry| entry.path().is_dir() && entry.path().join("__init__.py").exists())
        }) {
            debug!("Project has src directory with __init__.py files");
            return true;
        }
    }

    // 4. Check for PEP 621 package indicators in [project] section
    let has_pep621_package_indicators = doc
        .get("project")
        .map(|project| {
            // Check for typical package indicators in PEP 621 format
            let has_urls = project.get("urls").is_some();
            let has_classifiers = project.get("classifiers").is_some();
            let has_keywords = project.get("keywords").is_some();

            // If it has multiple of these fields, it's likely a package
            (has_urls as u8) + (has_classifiers as u8) + (has_keywords as u8) >= 2
        })
        .unwrap_or(false);

    if has_pep621_package_indicators {
        debug!("Project has PEP 621 package indicators");
        return true;
    }

    // 5. Check for existing build-system in old pyproject.toml
    let has_build_system = doc.get("build-system").is_some();
    if has_build_system {
        debug!("Project already has a build-system section");
        return true;
    }

    debug!("No package indicators found, treating as application");
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn setup_test_environment(
        old_content: &str,
        new_content: &str,
        create_setup_py: bool,
        create_src_init: bool,
    ) -> (TempDir, DocumentMut, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        fs::write(project_dir.join("old.pyproject.toml"), old_content).unwrap();
        fs::write(project_dir.join("pyproject.toml"), new_content).unwrap();

        if create_setup_py {
            fs::write(project_dir.join("setup.py"), "# Test setup.py").unwrap();
        }

        if create_src_init {
            let src_dir = project_dir.join("src");
            let pkg_dir = src_dir.join("test_pkg");
            fs::create_dir_all(&pkg_dir).unwrap();
            fs::write(pkg_dir.join("__init__.py"), "# Test init file").unwrap();
        }

        let doc = new_content.parse::<DocumentMut>().unwrap();
        (temp_dir, doc, project_dir)
    }

    #[test]
    fn test_poetry_to_hatchling_conversion_with_existing_build_system() {
        let old_content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[build-system]
requires = ["poetry-core"]
build-backend = "poetry.core.masonry.api"
"#;

        let new_content = r#"
[project]
name = "test-project"
version = "0.1.0"
"#;

        let (_temp_dir, mut doc, project_dir) =
            setup_test_environment(old_content, new_content, false, false);

        let result = update_build_system(&mut doc, &project_dir).unwrap();
        assert!(result);

        let build_system = doc.get("build-system").unwrap();
        let requires = build_system.get("requires").unwrap().as_array().unwrap();
        let first_req = requires.get(0).unwrap().as_str().unwrap();
        assert_eq!(first_req, "hatchling");

        let backend = build_system.get("build-backend").unwrap().as_str().unwrap();
        assert_eq!(backend, "hatchling.build");
    }

    #[test]
    fn test_poetry_to_hatchling_with_poetry_package_config() {
        let old_content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.packages]
packages = [
     { from = "src" },
]
"#;

        let new_content = r#"
[project]
name = "test-project"
version = "0.1.0"
"#;

        let (_temp_dir, mut doc, project_dir) =
            setup_test_environment(old_content, new_content, false, false);

        let result = update_build_system(&mut doc, &project_dir).unwrap();
        assert!(result);

        let build_system = doc.get("build-system").unwrap();
        let requires = build_system.get("requires").unwrap().as_array().unwrap();
        let first_req = requires.get(0).unwrap().as_str().unwrap();
        assert_eq!(first_req, "hatchling");
    }

    #[test]
    fn test_poetry_to_hatchling_with_setup_py() {
        let old_content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"
"#;

        let new_content = r#"
[project]
name = "test-project"
version = "0.1.0"
"#;

        let (_temp_dir, mut doc, project_dir) =
            setup_test_environment(old_content, new_content, true, false);

        let result = update_build_system(&mut doc, &project_dir).unwrap();
        assert!(result);

        let build_system = doc.get("build-system").unwrap();
        let backend = build_system.get("build-backend").unwrap().as_str().unwrap();
        assert_eq!(backend, "hatchling.build");
    }

    #[test]
    fn test_poetry_to_hatchling_with_src_init() {
        let old_content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"
"#;

        let new_content = r#"
[project]
name = "test-project"
version = "0.1.0"
"#;

        let (_temp_dir, mut doc, project_dir) =
            setup_test_environment(old_content, new_content, false, true);

        let result = update_build_system(&mut doc, &project_dir).unwrap();
        assert!(result);

        let build_system = doc.get("build-system").unwrap();
        let backend = build_system.get("build-backend").unwrap().as_str().unwrap();
        assert_eq!(backend, "hatchling.build");
    }

    #[test]
    fn test_poetry_to_hatchling_with_pep621_indicators() {
        let old_content = r#"
[project]
name = "test-project"
version = "0.1.0"
description = "A test project"
classifiers = ["Programming Language :: Python"]
keywords = ["test", "project"]
urls = { repository = "https://github.com/user/repo" }
"#;

        let new_content = r#"
[project]
name = "test-project"
version = "0.1.0"
"#;

        let (_temp_dir, mut doc, project_dir) =
            setup_test_environment(old_content, new_content, false, false);

        let result = update_build_system(&mut doc, &project_dir).unwrap();
        assert!(result);

        let build_system = doc.get("build-system").unwrap();
        let backend = build_system.get("build-backend").unwrap().as_str().unwrap();
        assert_eq!(backend, "hatchling.build");
    }

    #[test]
    fn test_no_build_system_for_application() {
        let old_content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"
description = "A simple application"
"#;

        let new_content = r#"
[project]
name = "test-project"
version = "0.1.0"
"#;

        let (_temp_dir, mut doc, project_dir) =
            setup_test_environment(old_content, new_content, false, false);

        let result = update_build_system(&mut doc, &project_dir).unwrap();
        assert!(!result);
        assert!(doc.get("build-system").is_none());
    }

    #[test]
    fn test_no_old_pyproject() {
        let new_content = r#"
[project]
name = "test-project"
version = "0.1.0"
"#;

        let temp_dir = TempDir::new().unwrap();
        let mut doc = new_content.parse::<DocumentMut>().unwrap();

        let result = update_build_system(&mut doc, temp_dir.path()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_determine_if_package_project() {
        // Test with package configuration
        let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"
packages = [
    { include = "src" }
]
"#;
        let doc = content.parse::<DocumentMut>().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let result = determine_if_package_project(&doc, temp_dir.path());
        assert!(
            result,
            "Should detect package project from Poetry packages config"
        );

        // Test with Poetry 2.0 format
        let content2 = r#"
[project]
name = "test-project"
version = "0.1.0"
packages = [
    { include = "src" }
]
"#;
        let doc2 = content2.parse::<DocumentMut>().unwrap();
        let result2 = determine_if_package_project(&doc2, temp_dir.path());
        assert!(
            result2,
            "Should detect package project from Poetry 2.0 packages config"
        );
    }

    #[test]
    fn test_single_package_include() {
        // Test with simple include format
        let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"
packages = [
    { include = "src" }
]
"#;
        let doc = content.parse::<DocumentMut>().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let result = determine_if_package_project(&doc, temp_dir.path());
        assert!(result, "Should detect package from single include format");
    }

    #[test]
    fn test_multiple_package_includes() {
        // Test with multiple includes
        let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"
packages = [
    { include = "src" },
    { include = "lib" }
]
"#;
        let doc = content.parse::<DocumentMut>().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let result = determine_if_package_project(&doc, temp_dir.path());
        assert!(result, "Should detect package from multiple includes");
    }
}
