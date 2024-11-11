use log::{info};
use std::path::Path;
use std::io::Write;

pub fn update_pyproject_toml(project_dir: &Path, extra_urls: &[String]) -> Result<(), String> {
    // Read the old pyproject.toml to get Poetry metadata
    let old_pyproject_path = project_dir.join("old.pyproject.toml");
    if !old_pyproject_path.exists() {
        return Ok(());
    }

    let old_content = std::fs::read_to_string(&old_pyproject_path)
        .map_err(|e| format!("Failed to read old.pyproject.toml: {}", e))?;

    // Parse the Poetry section manually
    let mut poetry_version = None;
    let mut poetry_description = None;
    let mut in_poetry_section = false;

    for line in old_content.lines() {
        let trimmed = line.trim();

        if trimmed == "[tool.poetry]" {
            in_poetry_section = true;
            continue;
        } else if trimmed.starts_with('[') && trimmed != "[tool.poetry]" {
            in_poetry_section = false;
            continue;
        }

        if !in_poetry_section {
            continue;
        }

        // Extract version and description from Poetry section
        if trimmed.starts_with("version = ") {
            poetry_version = Some(trimmed.split_once('=').unwrap().1.trim().trim_matches('"'));
        } else if trimmed.starts_with("description = ") {
            poetry_description = Some(trimmed.split_once('=').unwrap().1.trim().trim_matches('"'));
        }
    }

    // Now update the new pyproject.toml
    let pyproject_path = project_dir.join("pyproject.toml");
    let content = std::fs::read_to_string(&pyproject_path)
        .map_err(|e| format!("Failed to read pyproject.toml: {}", e))?;

    let mut lines: Vec<String> = Vec::new();
    let mut in_project_section = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "[project]" {
            in_project_section = true;
            lines.push(line.to_string());
            continue;
        } else if trimmed.starts_with('[') {
            in_project_section = false;
        }

        if !in_project_section {
            lines.push(line.to_string());
            continue;
        }

        // Update version and description in the project section
        if trimmed.starts_with("version = ") && poetry_version.is_some() {
            lines.push(format!("version = \"{}\"", poetry_version.unwrap()));
        } else if trimmed.starts_with("description = ") && poetry_description.is_some() {
            lines.push(format!("description = \"{}\"", poetry_description.unwrap()));
        } else {
            lines.push(line.to_string());
        }
    }

    // Write the updated content back
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&pyproject_path)
        .map_err(|e| format!("Failed to open pyproject.toml for writing: {}", e))?;

    for line in lines {
        writeln!(file, "{}", line)
            .map_err(|e| format!("Failed to write line to pyproject.toml: {}", e))?;
    }

    // Handle extra URLs in a new section if needed
    if !extra_urls.is_empty() {
        let uv_section = format!(
            "\n[tool.uv]\nextra-index-url = {}\n",
            serde_json::to_string(extra_urls).map_err(|e| format!("Failed to serialize extra URLs: {}", e))?
        );

        file.write_all(uv_section.as_bytes())
            .map_err(|e| format!("Failed to write uv section: {}", e))?;
    }

    if poetry_version.is_some() || poetry_description.is_some() {
        info!("Successfully updated project metadata from Poetry configuration");
    }

    Ok(())
}