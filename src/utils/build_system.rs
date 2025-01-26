use log::debug;
use std::path::Path;
use toml_edit::{DocumentMut, Item, Table, Value};

/// Updates the build system configuration in pyproject.toml
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
    debug!("Checking for Poetry build system to migrate");
    let old_pyproject_path = project_dir.join("old.pyproject.toml");
    if !old_pyproject_path.exists() {
        return Ok(false);
    }

    // Read old pyproject.toml to check if it was a Poetry project
    let old_content = std::fs::read_to_string(&old_pyproject_path)
        .map_err(|e| format!("Failed to read old.pyproject.toml: {}", e))?;

    let old_doc = old_content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse old.pyproject.toml: {}", e))?;

    let was_poetry_project = old_doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|poetry| poetry.get("packages"))
        .and_then(|packages| packages.as_array())
        .is_some_and(|packages| {
            packages.iter().any(|pkg| {
                if let Some(table) = pkg.as_inline_table() {
                    (table.get("from").and_then(|f| f.as_str()) == Some("src"))
                        || (table.get("include").and_then(|i| i.as_str()).is_some()
                            && table.get("from").and_then(|f| f.as_str()) == Some("src"))
                } else {
                    false
                }
            })
        });

    let has_poetry_build_system = old_doc
        .get("build-system")
        .and_then(|bs| bs.get("requires"))
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().any(|v| v.as_str() == Some("poetry-core")))
        .unwrap_or(false);

    if !was_poetry_project && !has_poetry_build_system {
        return Ok(false);
    }

    debug!("Converting Poetry build system to Hatchling");

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn setup_test_environment(
        old_content: &str,
        new_content: &str,
    ) -> (TempDir, DocumentMut, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        fs::write(project_dir.join("old.pyproject.toml"), old_content).unwrap();
        fs::write(project_dir.join("pyproject.toml"), new_content).unwrap();

        let doc = new_content.parse::<DocumentMut>().unwrap();
        (temp_dir, doc, project_dir)
    }

    #[test]
    fn test_poetry_to_hatchling_conversion() {
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

        let (_temp_dir, mut doc, project_dir) = setup_test_environment(old_content, new_content);

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
    fn test_poetry_to_hatchling_with_just_from_src_package_config() {
        let old_content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.packages]
packages = [
     { from = "src" },
]

[build-system]
requires = ["poetry-core"]
build-backend = "poetry.core.masonry.api"
"#;

        let new_content = r#"
[project]
name = "test-project"
version = "0.1.0"
"#;

        let (_temp_dir, mut doc, project_dir) = setup_test_environment(old_content, new_content);

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
    fn test_poetry_to_hatchling_with_include_and_from_src_package_config() {
        let old_content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.packages]
packages = [
     { include = "my_package", from = "src" },
]

[build-system]
requires = ["poetry-core"]
build-backend = "poetry.core.masonry.api"
"#;

        let new_content = r#"
[project]
name = "test-project"
version = "0.1.0"
"#;

        let (_temp_dir, mut doc, project_dir) = setup_test_environment(old_content, new_content);

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
    fn test_no_conversion_for_non_poetry() {
        let old_content = r#"
[project]
name = "test-project"
version = "0.1.0"

[build-system]
requires = ["setuptools"]
build-backend = "setuptools.build_meta"
"#;

        let new_content = r#"
[project]
name = "test-project"
version = "0.1.0"
"#;

        let (_temp_dir, mut doc, project_dir) = setup_test_environment(old_content, new_content);

        let result = update_build_system(&mut doc, &project_dir).unwrap();
        assert!(!result);
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
}
