use crate::utils::toml::{read_toml, update_section, write_toml};
use log::{debug, info};
use std::path::Path;
use toml_edit::{Array, Formatted, Item, Table, Value};

pub fn update_pyproject_toml(project_dir: &Path, extra_urls: &[String]) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let old_pyproject_path = project_dir.join("old.pyproject.toml");

    if !old_pyproject_path.exists() {
        return Ok(());
    }

    let doc = read_toml(&old_pyproject_path)?;
    let mut new_doc = read_toml(&pyproject_path)?;

    if let Some(tool) = doc.get("tool") {
        if let Some(poetry) = tool.get("poetry") {
            if let Some(version) = poetry.get("version") {
                update_section(&mut new_doc, &["project", "version"], version.clone());
            }

            if let Some(description) = poetry.get("description") {
                update_section(
                    &mut new_doc,
                    &["project", "description"],
                    description.clone(),
                );
            }
        }
    }

    if !extra_urls.is_empty() {
        let mut array = Array::new();
        for url in extra_urls {
            array.push(Value::String(Formatted::new(url.to_string())));
        }
        update_section(
            &mut new_doc,
            &["tool", "uv", "extra-index-url"],
            Item::Value(Value::Array(array)),
        );
    }

    let mut doc = read_toml(&pyproject_path)?;
    write_toml(&pyproject_path, &mut doc)?;
    Ok(())
}

pub fn update_description(project_dir: &Path, description: &str) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let mut doc = read_toml(&pyproject_path)?;

    debug!("Updating project description");
    update_section(
        &mut doc,
        &["project", "description"],
        Item::Value(Value::String(Formatted::new(description.to_string()))),
    );

    let mut doc = read_toml(&pyproject_path)?;
    write_toml(&pyproject_path, &mut doc)?;
    info!("Successfully updated project description");
    Ok(())
}

pub fn update_url(project_dir: &Path, url: &str) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let mut doc = read_toml(&pyproject_path)?;

    debug!("Updating project URL");
    let mut urls_table = Table::new();
    urls_table.insert(
        "repository",
        Item::Value(Value::String(Formatted::new(url.to_string()))),
    );
    update_section(&mut doc, &["project", "urls"], Item::Table(urls_table));

    let mut doc = read_toml(&pyproject_path)?;
    write_toml(&pyproject_path, &mut doc)?;

    info!("Successfully updated project URL");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_update_description() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // Create initial pyproject.toml
        let initial_content = r#"[project]
name = "test"
version = "0.1.0"
"#;
        fs::write(project_dir.join("pyproject.toml"), initial_content).unwrap();

        // Update description
        let description = "A test project description";
        update_description(&project_dir, description).unwrap();

        // Read updated content
        let content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
        assert!(content.contains(&format!("description = \"{}\"", description)));
    }

    #[test]
    fn test_update_url() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // Create initial pyproject.toml
        let initial_content = r#"[project]
name = "test"
version = "0.1.0"
"#;
        fs::write(project_dir.join("pyproject.toml"), initial_content).unwrap();

        // Update URL
        let url = "https://github.com/example/test";
        update_url(&project_dir, url).unwrap();

        // Read updated content
        let content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
        assert!(content.contains(&format!("repository = \"{}\"", url)));
    }

    #[test]
    fn test_update_pyproject_with_extra_urls() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        // Create old.pyproject.toml
        let old_content = r#"[tool.poetry]
name = "test"
version = "1.0.0"
description = "Old description"
"#;
        fs::write(project_dir.join("old.pyproject.toml"), old_content).unwrap();

        // Create new pyproject.toml
        let new_content = r#"[project]
name = "test"
version = "0.1.0"
"#;
        fs::write(project_dir.join("pyproject.toml"), new_content).unwrap();

        // Update with extra URLs
        let extra_urls = vec![
            "https://test.pypi.org/simple/".to_string(),
            "https://custom.index/simple/".to_string(),
        ];
        update_pyproject_toml(&project_dir, &extra_urls).unwrap();

        // Read updated content
        let content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
        assert!(content.contains("version = \"1.0.0\""));
        assert!(content.contains("description = \"Old description\""));
        assert!(content.contains("https://test.pypi.org/simple/"));
        assert!(content.contains("https://custom.index/simple/"));
    }
}
