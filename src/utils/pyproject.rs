use crate::utils::toml::{read_toml, update_section, write_toml};
use log::{debug, info};
use std::path::Path;
use toml_edit::{Array, DocumentMut, Formatted, Item, Table, Value};

/// Reads a TOML file and returns its content as a DocumentMut.
///
/// This function delegates to the utility function in toml.rs to avoid
/// code duplication and ensure consistent error handling.
fn read_and_parse_toml(path: &Path) -> Result<DocumentMut, String> {
    read_toml(path)
}

pub fn update_pyproject_toml(project_dir: &Path, _extra_urls: &[String]) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let old_pyproject_path = project_dir.join("old.pyproject.toml");

    if !old_pyproject_path.exists() {
        return Ok(());
    }

    let old_doc = read_and_parse_toml(&old_pyproject_path)?;
    let mut new_doc = read_and_parse_toml(&pyproject_path)?;

    // Try Poetry 2.0 format first (project section)
    if let Some(project) = old_doc.get("project") {
        if let Some(description) = project.get("description") {
            update_section(
                &mut new_doc,
                &["project", "description"],
                description.clone(),
            );
        }
        if let Some(version) = project.get("version") {
            update_section(&mut new_doc, &["project", "version"], version.clone());
        }
    }

    // Fallback to Poetry 1.0 format (tool.poetry section)
    if let Some(tool) = old_doc.get("tool") {
        if let Some(poetry) = tool.get("poetry") {
            if let Some(description) = poetry.get("description") {
                update_section(
                    &mut new_doc,
                    &["project", "description"],
                    description.clone(),
                );
            }
            if let Some(version) = poetry.get("version") {
                update_section(&mut new_doc, &["project", "version"], version.clone());
            }
        }
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

pub fn migrate_poetry_scripts(doc: &DocumentMut) -> Option<Table> {
    // First try looking in the old Poetry style (tool.poetry.scripts)
    if let Some(poetry_scripts) = doc
        .get("tool")
        .and_then(|tool| tool.get("poetry"))
        .and_then(|poetry| poetry.get("scripts"))
        .and_then(|scripts| scripts.as_table())
    {
        let mut scripts_table = Table::new();
        scripts_table.set_implicit(true);

        for (script_name, script_value) in poetry_scripts.iter() {
            if let Some(script_str) = script_value.as_str() {
                let sanitized_name = sanitize_script_name(script_name);
                let converted_script = convert_script_format(script_str);
                scripts_table.insert(
                    &sanitized_name,
                    toml_edit::Item::Value(Value::String(Formatted::new(converted_script))),
                );
            }
        }

        if !scripts_table.is_empty() {
            return Some(scripts_table);
        }
    }

    // Then check for Poetry 2.0 style (project.scripts)
    if let Some(project_scripts) = doc
        .get("project")
        .and_then(|project| project.get("scripts"))
        .and_then(|scripts| scripts.as_table())
    {
        let mut scripts_table = Table::new();
        scripts_table.set_implicit(true);

        for (script_name, script_value) in project_scripts.iter() {
            if let Some(script_str) = script_value.as_str() {
                let sanitized_name = sanitize_script_name(script_name);
                let converted_script = convert_script_format(script_str);
                scripts_table.insert(
                    &sanitized_name,
                    toml_edit::Item::Value(Value::String(Formatted::new(converted_script))),
                );
            }
        }

        if !scripts_table.is_empty() {
            return Some(scripts_table);
        }
    }

    None
}

pub fn update_scripts(project_dir: &Path) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let old_pyproject_path = project_dir.join("old.pyproject.toml");

    // First read the new pyproject.toml
    let mut doc = read_and_parse_toml(&pyproject_path)?;

    // Check and sanitize any existing scripts in the new pyproject.toml
    if let Some(project) = doc.get_mut("project") {
        if let Some(scripts) = project.get("scripts").and_then(|s| s.as_table()) {
            let mut sanitized_scripts = Table::new();
            for (name, value) in scripts.iter() {
                let sanitized_name = sanitize_script_name(name);
                sanitized_scripts.insert(&sanitized_name, value.clone());
            }
            if let Some(table) = project.as_table_mut() {
                table.remove("scripts");
                table.insert("scripts", Item::Table(sanitized_scripts));
            }
        }
    }

    // Handle migration from old pyproject.toml if it exists
    if old_pyproject_path.exists() {
        let old_doc = read_and_parse_toml(&old_pyproject_path)?;
        if let Some(scripts_table) = migrate_poetry_scripts(&old_doc) {
            update_section(
                &mut doc,
                &["project", "scripts"],
                Item::Table(scripts_table),
            );

            // Remove the old scripts section if it exists
            if let Some(tool) = doc.get_mut("tool") {
                if let Some(poetry) = tool.get_mut("poetry") {
                    if let Some(table) = poetry.as_table_mut() {
                        table.remove("scripts");
                    }
                }
            }
        }
    }

    write_toml(&pyproject_path, &mut doc)?;
    info!("Successfully processed scripts in pyproject.toml");

    Ok(())
}

fn sanitize_script_name(name: &str) -> String {
    // List of reserved names that should be modified
    const RESERVED_NAMES: [&str; 1] = ["python"];

    let sanitized = name.trim().to_lowercase();
    if RESERVED_NAMES.contains(&sanitized.as_str()) {
        let new_name = format!("{}_script", sanitized);
        log::warn!(
            "Script name '{}' is reserved - automatically renamed to '{}'",
            name,
            new_name
        );
        new_name
    } else {
        name.to_string()
    }
}

fn convert_script_format(poetry_script: &str) -> String {
    let script = poetry_script.trim_matches(|c| c == '\'' || c == '"');
    sanitize_script_name(script)
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
                    && !section_value.as_table().is_some_and(|t| t.is_empty())
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

pub fn update_uv_indices(project_dir: &Path, sources: &[(String, String)]) -> Result<(), String> {
    if sources.is_empty() {
        return Ok(());
    }

    let pyproject_path = project_dir.join("pyproject.toml");
    let mut doc = read_and_parse_toml(&pyproject_path)?;

    let index_array: Array = sources
        .iter()
        .map(|(name, url)| {
            let mut index_table = toml_edit::InlineTable::new();
            index_table.insert("name", Value::String(Formatted::new(name.clone())));
            index_table.insert("url", Value::String(Formatted::new(url.clone())));
            Value::InlineTable(index_table)
        })
        .collect();

    update_section(
        &mut doc,
        &["tool", "uv", "index"],
        Item::Value(Value::Array(index_array)),
    );
    write_toml(&pyproject_path, &mut doc)?;
    Ok(())
}

/// Updates the uv.index section with index URLs from command line
pub fn update_uv_indices_from_urls(project_dir: &Path, urls: &[String]) -> Result<(), String> {
    if urls.is_empty() {
        return Ok(());
    }

    let pyproject_path = project_dir.join("pyproject.toml");
    let mut doc = read_and_parse_toml(&pyproject_path)?;

    let index_array: Array = urls
        .iter()
        .enumerate()
        .map(|(i, url)| {
            let mut index_table = toml_edit::InlineTable::new();
            // Generate a simple name like "custom1", "custom2" etc.
            let name = format!("custom{}", i + 1);
            index_table.insert("name", Value::String(Formatted::new(name)));
            index_table.insert("url", Value::String(Formatted::new(url.clone())));
            Value::InlineTable(index_table)
        })
        .collect();

    update_section(
        &mut doc,
        &["tool", "uv", "index"],
        Item::Value(Value::Array(index_array)),
    );
    write_toml(&pyproject_path, &mut doc)?;

    info!("Added {} custom index URLs to pyproject.toml", urls.len());
    Ok(())
}

/// Extracts Poetry source configurations from a pyproject.toml file
///
/// This function retrieves all package sources (index URLs) configured in Poetry,
/// supporting both array-of-tables and array formats.
pub fn extract_poetry_sources(project_dir: &Path) -> Result<Vec<(String, String)>, String> {
    let old_pyproject_path = project_dir.join("old.pyproject.toml");
    if !old_pyproject_path.exists() {
        return Ok(Vec::new());
    }

    let doc = read_and_parse_toml(&old_pyproject_path)?;
    let mut sources = Vec::new();

    // Try to extract sources from the TOML document
    if let Some(poetry_tool) = doc.get("tool").and_then(|tool| tool.get("poetry")) {
        // First try to extract as array-of-tables (common format)
        if let Some(array_of_tables) = poetry_tool
            .get("source")
            .and_then(|source| source.as_array_of_tables())
        {
            for table in array_of_tables {
                if let (Some(name), Some(url)) = (
                    table.get("name").and_then(|n| n.as_str()),
                    table.get("url").and_then(|u| u.as_str()),
                ) {
                    sources.push((name.to_string(), url.to_string()));
                }
            }
        }

        // If no sources were found, try parsing as regular array
        if sources.is_empty() {
            // Fall back to parsing as regular TOML Value
            if let Ok(parsed_toml) = toml::from_str::<toml::Value>(&doc.to_string()) {
                if let Some(source_array) = parsed_toml
                    .get("tool")
                    .and_then(|tool| tool.get("poetry"))
                    .and_then(|poetry| poetry.get("source"))
                    .and_then(|source| source.as_array())
                {
                    for source in source_array {
                        if let (Some(name), Some(url)) = (
                            source.get("name").and_then(|n| n.as_str()),
                            source.get("url").and_then(|u| u.as_str()),
                        ) {
                            sources.push((name.to_string(), url.to_string()));
                        }
                    }
                }
            }
        }
    }

    Ok(sources)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_name_sanitization() {
        assert_eq!(sanitize_script_name("test"), "test");
        assert_eq!(sanitize_script_name("python"), "python_script");
        assert_eq!(sanitize_script_name("PYTHON"), "python_script");
        assert_eq!(sanitize_script_name(" python "), "python_script");
        assert_eq!(sanitize_script_name("other"), "other");
    }

    #[test]
    fn test_convert_script_format() {
        assert_eq!(convert_script_format("\"test-command\""), "test-command");
        assert_eq!(convert_script_format("'python-run'"), "python-run");
        assert_eq!(convert_script_format("python"), "python_script");
        assert_eq!(convert_script_format("\"python\""), "python_script");
    }
}
