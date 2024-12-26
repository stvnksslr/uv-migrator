use crate::utils::toml::{read_toml, update_section, write_toml};
use log::{debug, info};
use std::path::Path;
use toml_edit::{Array, Formatted, Item, Value};

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
