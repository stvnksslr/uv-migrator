use crate::utils::toml::{read_toml, update_section, write_toml};
use log::{debug, info};
use std::path::Path;
use toml_edit::{Item, Table};

pub fn append_tool_sections(project_dir: &Path) -> Result<(), String> {
    let old_pyproject_path = project_dir.join("old.pyproject.toml");
    let pyproject_path = project_dir.join("pyproject.toml");

    if !old_pyproject_path.exists() {
        debug!("old.pyproject.toml not found. Skipping tool section migration.");
        return Ok(());
    }

    debug!("Reading old and new pyproject.toml files");
    let old_doc = read_toml(&old_pyproject_path)?;
    let mut new_doc = read_toml(&pyproject_path)?;

    // First collect all valid tool sections to copy
    let mut sections_to_copy: Vec<(String, Item)> = Vec::new();

    if let Some(old_tool) = old_doc.get("tool") {
        if let Some(old_tool_table) = old_tool.as_table() {
            let existing_sections: Vec<String> = new_doc
                .get("tool")
                .and_then(|t| t.as_table())
                .map(|t| t.iter().map(|(k, _)| k.to_string()).collect())
                .unwrap_or_default();

            for (section_name, section_value) in old_tool_table.iter() {
                if section_name != "poetry"
                    && !existing_sections.contains(&section_name.to_string())
                {
                    debug!("Found tool section to copy: {}", section_name);
                    sections_to_copy.push((section_name.to_string(), section_value.clone()));
                }
            }
        }
    }

    // Remove any empty tool section before adding new content
    if let Some(tool) = new_doc.get("tool") {
        if tool.as_table().map_or(false, |t| t.is_empty()) {
            debug!("Removing empty [tool] section before adding new content");
            new_doc.remove("tool");
        }
    }

    // Only create tool section if we have sections to copy
    if !sections_to_copy.is_empty() {
        debug!("Creating/updating tool sections");

        // Create new tool table if needed
        if !new_doc.contains_key("tool") {
            let mut tool_table = Table::new();
            // Mark the table as requiring brackets
            tool_table.set_implicit(false);
            update_section(&mut new_doc, &["tool"], Item::Table(tool_table));
        }

        // Copy all collected sections
        for (section_name, section_value) in sections_to_copy {
            update_section(&mut new_doc, &["tool", &section_name], section_value);
            info!("Copied tool.{} section", section_name);
        }
    }

    write_toml(&pyproject_path, &mut new_doc)?;

    info!("Successfully managed tool sections in new pyproject.toml");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_files(old_content: &str, new_content: &str) -> (TempDir, std::path::PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        fs::write(project_dir.join("old.pyproject.toml"), old_content).unwrap();
        fs::write(project_dir.join("pyproject.toml"), new_content).unwrap();

        (temp_dir, project_dir)
    }

    #[test]
    fn test_handle_empty_tool_section() {
        let old_content = r#"
[tool.black]
line-length = 100
target-version = ["py37"]

[tool.isort]
profile = "black"
"#;

        let new_content = r#"
[project]
name = "test"
version = "0.1.0"
description = "Test project"

[tool]

[tool.mypy]
ignore_missing_imports = true
"#;

        let (_temp_dir, project_dir) = setup_test_files(old_content, new_content);
        append_tool_sections(&project_dir).unwrap();

        let final_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();

        // Check that all tool sections are present
        assert!(final_content.contains("[tool.black]"));
        assert!(final_content.contains("line-length = 100"));
        assert!(final_content.contains("[tool.isort]"));
        assert!(final_content.contains("profile = \"black\""));
        assert!(final_content.contains("[tool.mypy]"));
        assert!(final_content.contains("ignore_missing_imports = true"));

        // Check that empty tool section is not present
        let lines: Vec<&str> = final_content.lines().collect();
        assert!(
            !lines.contains(&"[tool]"),
            "Empty [tool] section should not exist"
        );
    }
}
