use crate::utils::toml::{read_toml, update_section, write_toml};
use log::{debug, info};
use std::{fs, path::Path};
use toml_edit::Table;
use toml_edit::{Array, DocumentMut, Formatted, Item, Value};

pub fn update_pyproject_toml(project_dir: &Path, extra_urls: &[String]) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let old_pyproject_path = project_dir.join("old.pyproject.toml");

    if !old_pyproject_path.exists() {
        return Ok(());
    }

    let old_doc = read_toml(&old_pyproject_path)?;
    let mut new_doc = read_toml(&pyproject_path)?;

    if let Some(tool) = old_doc.get("tool") {
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

    write_toml(&pyproject_path, &mut new_doc)?;
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

    write_toml(&pyproject_path, &mut doc)?;
    info!("Successfully updated project description");
    Ok(())
}

pub fn update_url(project_dir: &Path, url: &str) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let mut doc = read_toml(&pyproject_path)?;

    let mut urls_table = toml_edit::InlineTable::new();
    urls_table.insert("repository", Value::String(Formatted::new(url.to_string())));

    update_section(
        &mut doc,
        &["project", "urls"],
        Item::Value(Value::InlineTable(urls_table)),
    );

    write_toml(&pyproject_path, &mut doc)?;
    info!("Successfully updated project URL");
    Ok(())
}

fn convert_script_format(poetry_script: &str) -> String {
    // Poetry format: 'package.module:function'
    // UV format: "package.module:function"
    poetry_script.trim_matches('\'').to_string()
}

pub fn migrate_poetry_scripts(doc: &DocumentMut) -> Option<Table> {
    let poetry_scripts = doc.get("tool")?.get("poetry")?.get("scripts")?.as_table()?;

    let mut scripts_table = Table::new();

    for (script_name, script_value) in poetry_scripts.iter() {
        if let Some(script_str) = script_value.as_str() {
            // Convert Poetry script format to UV format
            let converted_script = convert_script_format(script_str);
            scripts_table.insert(
                script_name,
                toml_edit::Item::Value(Value::String(Formatted::new(converted_script))),
            );
        }
    }

    if !scripts_table.is_empty() {
        Some(scripts_table)
    } else {
        None
    }
}

pub fn update_scripts(project_dir: &Path) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let content = fs::read_to_string(&pyproject_path)
        .map_err(|e| format!("Failed to read pyproject.toml: {}", e))?;

    let doc = content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse TOML: {}", e))?;

    if let Some(scripts_table) = migrate_poetry_scripts(&doc) {
        let mut new_doc = doc.clone();

        // Remove old poetry scripts section
        if let Some(tool) = new_doc.get_mut("tool") {
            if let Some(poetry) = tool.get_mut("poetry") {
                if let Some(table) = poetry.as_table_mut() {
                    table.remove("scripts");
                }
            }
        }

        // Add new scripts section
        update_section(
            &mut new_doc,
            &["project", "scripts"],
            Item::Table(scripts_table),
        );

        write_toml(&pyproject_path, &mut new_doc)?;
        info!("Successfully migrated Poetry scripts to UV format");
    }

    Ok(())
}

pub fn update_project_version(project_dir: &Path, version: &str) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let mut doc = read_toml(&pyproject_path)?;

    debug!("Updating project version to {}", version);
    update_section(
        &mut doc,
        &["project", "version"],
        Item::Value(Value::String(Formatted::new(version.to_string()))),
    );

    write_toml(&pyproject_path, &mut doc)?;
    info!("Successfully updated project version");
    Ok(())
}

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

    // Only proceed if there are tool sections to migrate
    if let Some(tool) = old_doc.get("tool") {
        if let Some(tool_table) = tool.as_table() {
            let existing_sections: Vec<String> = new_doc
                .get("tool")
                .and_then(|t| t.as_table())
                .map(|t| t.iter().map(|(k, _)| k.to_string()).collect())
                .unwrap_or_default();

            // Track if any sections were actually copied
            let mut sections_copied = false;

            // Copy each non-poetry tool section that doesn't already exist
            for (section_name, section_value) in tool_table.iter() {
                if section_name != "poetry"
                    && !existing_sections.contains(&section_name.to_string())
                    && !section_value.as_table().map_or(false, |t| t.is_empty())
                {
                    debug!("Copying tool section: {}", section_name);
                    update_section(&mut new_doc, &["tool", section_name], section_value.clone());
                    sections_copied = true;
                }
            }

            if sections_copied {
                write_toml(&pyproject_path, &mut new_doc)?;
                info!("Successfully managed tool sections in new pyproject.toml");
            } else {
                debug!("No tool sections needed migration");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use tempfile::TempDir;
    use toml_edit::DocumentMut;

    fn create_test_pyproject(content: &str) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();
        let pyproject_path = project_dir.join("pyproject.toml");
        fs::write(&pyproject_path, content).unwrap();
        (temp_dir, project_dir)
    }

    #[test]
    fn test_basic_script_migration() {
        let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.scripts]
cli = "my_package.cli:main"
serve = "my_package.server:run_server"
"#;
        let (_temp_dir, project_dir) = create_test_pyproject(content);
        update_scripts(&project_dir).unwrap();

        let new_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
        let doc = new_content.parse::<DocumentMut>().unwrap();

        assert!(doc
            .get("tool")
            .unwrap()
            .get("poetry")
            .unwrap()
            .get("scripts")
            .is_none());

        let scripts = doc
            .get("project")
            .unwrap()
            .get("scripts")
            .unwrap()
            .as_table()
            .unwrap();
        assert_eq!(
            scripts.get("cli").unwrap().as_str().unwrap(),
            "my_package.cli:main"
        );
        assert_eq!(
            scripts.get("serve").unwrap().as_str().unwrap(),
            "my_package.server:run_server"
        );
    }

    #[test]
    fn test_script_with_single_quotes() {
        let content = r#"
[tool.poetry.scripts]
start = 'package.module:func'
"#;
        let (_temp_dir, project_dir) = create_test_pyproject(content);
        update_scripts(&project_dir).unwrap();

        let new_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
        let doc = new_content.parse::<DocumentMut>().unwrap();

        let scripts = doc
            .get("project")
            .unwrap()
            .get("scripts")
            .unwrap()
            .as_table()
            .unwrap();
        assert_eq!(
            scripts.get("start").unwrap().as_str().unwrap(),
            "package.module:func"
        );
    }

    #[test]
    fn test_multiple_complex_scripts() {
        let content = r#"
[tool.poetry.scripts]
cli = "package.commands.cli:main_func"
web = "package.web.server:start_server"
worker = "package.workers.background:process_queue"
"#;
        let (_temp_dir, project_dir) = create_test_pyproject(content);
        update_scripts(&project_dir).unwrap();

        let new_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
        let doc = new_content.parse::<DocumentMut>().unwrap();

        let scripts = doc
            .get("project")
            .unwrap()
            .get("scripts")
            .unwrap()
            .as_table()
            .unwrap();
        assert_eq!(
            scripts.get("cli").unwrap().as_str().unwrap(),
            "package.commands.cli:main_func"
        );
        assert_eq!(
            scripts.get("web").unwrap().as_str().unwrap(),
            "package.web.server:start_server"
        );
        assert_eq!(
            scripts.get("worker").unwrap().as_str().unwrap(),
            "package.workers.background:process_queue"
        );
    }

    #[test]
    fn test_empty_scripts_section() {
        let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.scripts]
"#;
        let (_temp_dir, project_dir) = create_test_pyproject(content);
        update_scripts(&project_dir).unwrap();

        let new_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
        let doc = new_content.parse::<DocumentMut>().unwrap();

        assert!(doc.get("project").and_then(|p| p.get("scripts")).is_none());
    }

    #[test]
    fn test_no_scripts_section() {
        let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"
"#;
        let (_temp_dir, project_dir) = create_test_pyproject(content);
        update_scripts(&project_dir).unwrap();

        let new_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
        let doc = new_content.parse::<DocumentMut>().unwrap();

        assert!(doc.get("project").and_then(|p| p.get("scripts")).is_none());
    }

    #[test]
    fn test_script_format_conversion() {
        let test_cases = vec![
            ("'package.module:func'", "package.module:func"),
            ("package.module:func", "package.module:func"),
            ("'module:main'", "module:main"),
            (
                "'deeply.nested.module:complex_func'",
                "deeply.nested.module:complex_func",
            ),
        ];

        for (input, expected) in test_cases {
            assert_eq!(convert_script_format(input), expected);
        }
    }

    #[test]
    fn test_preserve_other_sections() {
        let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.scripts]
cli = "package.cli:main"

[build-system]
requires = ["poetry-core"]
build-backend = "poetry.core.masonry.api"

[tool.other]
setting = "value"
"#;
        let (_temp_dir, project_dir) = create_test_pyproject(content);
        update_scripts(&project_dir).unwrap();

        let new_content = fs::read_to_string(project_dir.join("pyproject.toml")).unwrap();
        let doc = new_content.parse::<DocumentMut>().unwrap();

        assert!(doc.get("build-system").is_some());
        assert!(doc.get("tool").unwrap().get("other").is_some());
        assert_eq!(
            doc.get("tool")
                .unwrap()
                .get("other")
                .unwrap()
                .get("setting")
                .unwrap()
                .as_str()
                .unwrap(),
            "value"
        );
    }
}
