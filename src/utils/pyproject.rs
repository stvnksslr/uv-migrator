use log::{debug, info};
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn update_pyproject_toml(project_dir: &Path, extra_urls: &[String]) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let old_pyproject_path = project_dir.join("old.pyproject.toml");

    debug!("Checking for old.pyproject.toml");
    if !old_pyproject_path.exists() {
        debug!("old.pyproject.toml not found, skipping metadata update");
        return Ok(());
    }

    debug!("Reading old pyproject.toml content");
    let old_content = fs::read_to_string(&old_pyproject_path)
        .map_err(|e| format!("Failed to read old.pyproject.toml: {}", e))?;

    let mut poetry_version = None;
    let mut poetry_description = None;
    let mut in_poetry_section = false;

    debug!("Parsing old pyproject.toml for metadata");
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

        if trimmed.starts_with("version = ") {
            poetry_version = Some(trimmed.split_once('=').unwrap().1.trim().trim_matches('"'));
            debug!("Found version: {:?}", poetry_version);
        } else if trimmed.starts_with("description = ") {
            poetry_description = Some(trimmed.split_once('=').unwrap().1.trim().trim_matches('"'));
            debug!("Found description: {:?}", poetry_description);
        }
    }

    debug!("Reading current pyproject.toml");
    let content = fs::read_to_string(&pyproject_path)
        .map_err(|e| format!("Failed to read pyproject.toml: {}", e))?;

    let mut lines: Vec<String> = Vec::new();
    let mut in_project_section = false;

    // Update metadata in the project section
    debug!("Updating project metadata");
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

        if trimmed.starts_with("version = ") && poetry_version.is_some() {
            lines.push(format!("version = \"{}\"", poetry_version.unwrap()));
            debug!("Updated version in project section");
        } else if trimmed.starts_with("description = ") && poetry_description.is_some() {
            lines.push(format!("description = \"{}\"", poetry_description.unwrap()));
            debug!("Updated description in project section");
        } else {
            lines.push(line.to_string());
        }
    }

    debug!("Opening pyproject.toml for writing");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&pyproject_path)
        .map_err(|e| format!("Failed to open pyproject.toml for writing: {}", e))?;

    // Write updated content
    debug!("Writing updated content to pyproject.toml");
    for line in lines {
        writeln!(file, "{}", line)
            .map_err(|e| format!("Failed to write line to pyproject.toml: {}", e))?;
    }

    // Add UV section with extra URLs if present
    if !extra_urls.is_empty() {
        debug!("Adding UV section with extra URLs");
        let uv_section = format!(
            "\n[tool.uv]\nextra-index-url = {}\n",
            serde_json::to_string(extra_urls)
                .map_err(|e| format!("Failed to serialize extra URLs: {}", e))?
        );
        file.write_all(uv_section.as_bytes())
            .map_err(|e| format!("Failed to write uv section: {}", e))?;
    }

    if poetry_version.is_some() || poetry_description.is_some() {
        info!("Successfully updated project metadata from Poetry configuration");
    }

    debug!("Successfully completed pyproject.toml update");
    Ok(())
}
