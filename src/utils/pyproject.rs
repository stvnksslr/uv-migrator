use crate::error::{Error, Result};
use crate::models::GitDependency;
use crate::utils::toml::{read_toml, update_section, write_toml};
use log::{debug, info};
use std::fs;
use std::path::Path;
use toml_edit::{Array, DocumentMut, Formatted, InlineTable, Item, Table, Value};

/// Updates the pyproject.toml with basic project metadata
pub fn update_pyproject_toml(project_dir: &Path, _extra_args: &[String]) -> Result<()> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let old_pyproject_path = project_dir.join("old.pyproject.toml");

    if !old_pyproject_path.exists() {
        debug!("No old.pyproject.toml found, skipping update");
        return Ok(());
    }

    let mut doc = read_toml(&pyproject_path)?;
    let old_doc = read_toml(&old_pyproject_path)?;

    // Transfer basic metadata if available
    if let Some(old_tool) = old_doc.get("tool") {
        if let Some(old_poetry) = old_tool.get("poetry") {
            // Transfer version if available
            if let Some(version) = old_poetry.get("version").and_then(|v| v.as_str()) {
                update_section(
                    &mut doc,
                    &["project", "version"],
                    Item::Value(Value::String(Formatted::new(version.to_string()))),
                );
            }

            // Transfer description if available
            if let Some(desc) = old_poetry.get("description").and_then(|d| d.as_str()) {
                update_section(
                    &mut doc,
                    &["project", "description"],
                    Item::Value(Value::String(Formatted::new(desc.to_string()))),
                );
            }
        }
    }

    // Also check Poetry 2.0 style
    if let Some(old_project) = old_doc.get("project") {
        if let Some(version) = old_project.get("version").and_then(|v| v.as_str()) {
            update_section(
                &mut doc,
                &["project", "version"],
                Item::Value(Value::String(Formatted::new(version.to_string()))),
            );
        }

        if let Some(desc) = old_project.get("description").and_then(|d| d.as_str()) {
            update_section(
                &mut doc,
                &["project", "description"],
                Item::Value(Value::String(Formatted::new(desc.to_string()))),
            );
        }
    }

    write_toml(&pyproject_path, &mut doc)?;
    Ok(())
}

/// Updates the project version in pyproject.toml
pub fn update_project_version(project_dir: &Path, version: &str) -> Result<()> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let mut doc = read_toml(&pyproject_path)?;

    update_section(
        &mut doc,
        &["project", "version"],
        Item::Value(Value::String(Formatted::new(version.to_string()))),
    );

    write_toml(&pyproject_path, &mut doc)?;
    info!("Updated project version to {}", version);
    Ok(())
}

/// Extracts Poetry package sources from old pyproject.toml
pub fn extract_poetry_sources(project_dir: &Path) -> Result<Vec<toml::Value>> {
    let old_pyproject_path = project_dir.join("old.pyproject.toml");
    if !old_pyproject_path.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&old_pyproject_path).map_err(|e| Error::FileOperation {
        path: old_pyproject_path.clone(),
        message: format!("Failed to read old.pyproject.toml: {}", e),
    })?;

    let old_doc: toml::Value = toml::from_str(&content).map_err(Error::TomlSerde)?;

    if let Some(sources) = old_doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("source"))
        .and_then(|s| s.as_array())
    {
        Ok(sources.clone())
    } else {
        Ok(vec![])
    }
}

/// Updates UV indices in pyproject.toml
pub fn update_uv_indices(project_dir: &Path, sources: &[toml::Value]) -> Result<()> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let mut doc = read_toml(&pyproject_path)?;

    let mut indices = Array::new();
    for source in sources {
        if let Some(url) = source.get("url").and_then(|u| u.as_str()) {
            let mut table = InlineTable::new();

            if let Some(name) = source.get("name").and_then(|n| n.as_str()) {
                table.insert("name", Value::String(Formatted::new(name.to_string())));
            }

            table.insert("url", Value::String(Formatted::new(url.to_string())));

            indices.push(Value::InlineTable(table));
        }
    }

    if !indices.is_empty() {
        update_section(
            &mut doc,
            &["tool", "uv", "index"],
            Item::Value(Value::Array(indices)),
        );
        write_toml(&pyproject_path, &mut doc)?;
        info!("Migrated {} package sources to UV indices", sources.len());
    }

    Ok(())
}

/// Updates UV indices from URLs
pub fn update_uv_indices_from_urls(project_dir: &Path, urls: &[String]) -> Result<()> {
    if urls.is_empty() {
        return Ok(());
    }

    let pyproject_path = project_dir.join("pyproject.toml");
    let mut doc = read_toml(&pyproject_path)?;

    let mut indices = Array::new();
    for (i, url) in urls.iter().enumerate() {
        let mut table = InlineTable::new();
        table.insert(
            "name",
            Value::String(Formatted::new(format!("extra-{}", i + 1))),
        );
        table.insert("url", Value::String(Formatted::new(url.clone())));
        indices.push(Value::InlineTable(table));
    }

    update_section(
        &mut doc,
        &["tool", "uv", "index"],
        Item::Value(Value::Array(indices)),
    );

    write_toml(&pyproject_path, &mut doc)?;
    info!("Added {} extra index URLs", urls.len());
    Ok(())
}

/// Appends tool sections from old pyproject.toml to new one
pub fn append_tool_sections(project_dir: &Path) -> Result<()> {
    let old_pyproject_path = project_dir.join("old.pyproject.toml");
    let pyproject_path = project_dir.join("pyproject.toml");

    if !old_pyproject_path.exists() {
        debug!("No old.pyproject.toml found, skipping tool section migration");
        return Ok(());
    }

    let old_doc = read_toml(&old_pyproject_path)?;
    let mut new_doc = read_toml(&pyproject_path)?;

    // Copy tool sections except poetry
    if let Some(old_tool) = old_doc.get("tool").and_then(|t| t.as_table()) {
        for (key, value) in old_tool.iter() {
            if key != "poetry" && !is_empty_section(value) {
                // Check if the section already exists in the new document
                let section_exists = new_doc.get("tool").and_then(|t| t.get(key)).is_some();

                if !section_exists {
                    let path = ["tool", key];
                    update_section(&mut new_doc, &path, value.clone());
                    debug!("Migrated tool.{} section", key);
                } else {
                    debug!("Skipping tool.{} section - already exists in target", key);
                }
            }
        }
    }

    write_toml(&pyproject_path, &mut new_doc)?;
    Ok(())
}

/// Checks if a TOML item is empty
fn is_empty_section(item: &Item) -> bool {
    match item {
        Item::Table(table) => table.is_empty() || table.iter().all(|(_, v)| is_empty_section(v)),
        Item::Value(value) => {
            if let Some(array) = value.as_array() {
                array.is_empty()
            } else {
                false
            }
        }
        Item::None => true,
        Item::ArrayOfTables(array) => array.is_empty(),
    }
}

/// Updates scripts section from Poetry to standard format
pub fn update_scripts(project_dir: &Path) -> Result<bool> {
    let old_pyproject_path = project_dir.join("old.pyproject.toml");
    let pyproject_path = project_dir.join("pyproject.toml");

    if !old_pyproject_path.exists() {
        return Ok(false);
    }

    let old_doc = read_toml(&old_pyproject_path)?;
    let mut new_doc = read_toml(&pyproject_path)?;

    // Check for Poetry scripts
    if let Some(scripts) = old_doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("scripts"))
        .and_then(|s| s.as_table())
    {
        if !scripts.is_empty() {
            let mut scripts_table = InlineTable::new();

            for (name, value) in scripts.iter() {
                if let Item::Value(Value::String(s)) = value {
                    scripts_table.insert(name, Value::String(s.clone()));
                }
            }

            if !scripts_table.is_empty() {
                update_section(
                    &mut new_doc,
                    &["project", "scripts"],
                    Item::Value(Value::InlineTable(scripts_table)),
                );
                write_toml(&pyproject_path, &mut new_doc)?;
                info!("Migrated {} scripts", scripts.len());
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// Extracts Poetry packages configuration
pub fn extract_poetry_packages(doc: &DocumentMut) -> Vec<String> {
    let mut packages = Vec::new();

    if let Some(poetry_packages) = doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("packages"))
        .and_then(|p| p.as_array())
    {
        for pkg in poetry_packages.iter() {
            if let Some(table) = pkg.as_inline_table() {
                if let Some(include) = table.get("include").and_then(|i| i.as_str()) {
                    packages.push(include.to_string());
                }
            } else if let Some(pkg_str) = pkg.as_str() {
                packages.push(pkg_str.to_string());
            }
        }
    }

    packages
}

/// Updates git dependencies in pyproject.toml
pub fn update_git_dependencies(project_dir: &Path, git_deps: &[GitDependency]) -> Result<()> {
    if git_deps.is_empty() {
        return Ok(());
    }

    let pyproject_path = project_dir.join("pyproject.toml");
    let mut doc = read_toml(&pyproject_path)?;

    for dep in git_deps {
        let mut source_table = Table::new();
        source_table.insert(
            "git",
            Item::Value(Value::String(Formatted::new(dep.git_url.clone()))),
        );

        if let Some(branch) = &dep.branch {
            source_table.insert(
                "branch",
                Item::Value(Value::String(Formatted::new(branch.clone()))),
            );
        }

        if let Some(tag) = &dep.tag {
            source_table.insert(
                "tag",
                Item::Value(Value::String(Formatted::new(tag.clone()))),
            );
        }

        if let Some(rev) = &dep.rev {
            source_table.insert(
                "rev",
                Item::Value(Value::String(Formatted::new(rev.clone()))),
            );
        }

        let path = ["tool", "uv", "sources", &dep.name];
        update_section(&mut doc, &path, Item::Table(source_table));
    }

    write_toml(&pyproject_path, &mut doc)?;
    info!("Migrated {} git dependencies", git_deps.len());
    Ok(())
}

/// Extracts project name from pyproject.toml
pub fn extract_project_name(project_dir: &Path) -> Result<Option<String>> {
    let pyproject_path = project_dir.join("pyproject.toml");
    if !pyproject_path.exists() {
        return Ok(None);
    }

    let doc = read_toml(&pyproject_path)?;

    if let Some(name) = doc
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
    {
        Ok(Some(name.to_string()))
    } else {
        Ok(None)
    }
}

/// Updates project description
pub fn update_description(project_dir: &Path, description: &str) -> Result<()> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let mut doc = read_toml(&pyproject_path)?;

    update_section(
        &mut doc,
        &["project", "description"],
        Item::Value(Value::String(Formatted::new(description.to_string()))),
    );

    write_toml(&pyproject_path, &mut doc)?;
    info!("Updated project description");
    Ok(())
}

/// Updates project URL
pub fn update_url(project_dir: &Path, url: &str) -> Result<()> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let mut doc = read_toml(&pyproject_path)?;

    let mut urls_table = InlineTable::new();
    urls_table.insert("repository", Value::String(Formatted::new(url.to_string())));

    update_section(
        &mut doc,
        &["project", "urls"],
        Item::Value(Value::InlineTable(urls_table)),
    );

    write_toml(&pyproject_path, &mut doc)?;
    info!("Updated project URL");
    Ok(())
}
