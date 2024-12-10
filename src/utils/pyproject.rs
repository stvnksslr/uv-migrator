use log::{debug, info, warn};
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn update_pyproject_toml(project_dir: &Path, extra_urls: &[String]) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let old_pyproject_path = project_dir.join("old.pyproject.toml");

    debug!("Starting metadata migration process");
    debug!(
        "Looking for old pyproject at: {}",
        old_pyproject_path.display()
    );

    if !old_pyproject_path.exists() {
        warn!(
            "old.pyproject.toml not found at {}",
            old_pyproject_path.display()
        );
        return Ok(());
    }

    debug!("Found old.pyproject.toml, reading content");
    let old_content = fs::read_to_string(&old_pyproject_path)
        .map_err(|e| format!("Failed to read old.pyproject.toml: {}", e))?;

    let mut poetry_version = None;
    let mut poetry_description = None;
    let mut in_poetry_section = false;

    debug!("Starting to parse old pyproject.toml");
    for line in old_content.lines() {
        let trimmed = line.trim();
        debug!("Processing line: {}", trimmed);

        if trimmed == "[tool.poetry]" {
            debug!("Found [tool.poetry] section");
            in_poetry_section = true;
            continue;
        } else if trimmed.starts_with('[') && trimmed != "[tool.poetry]" {
            if in_poetry_section {
                debug!("Leaving poetry section");
            }
            in_poetry_section = false;
            continue;
        }

        if !in_poetry_section {
            continue;
        }

        // Enhanced parsing of version and description
        if let Some(version) = parse_toml_value(trimmed, "version") {
            poetry_version = Some(version.clone());
            debug!("Found version in old file: {}", version);
        } else if let Some(description) = parse_toml_value(trimmed, "description") {
            poetry_description = Some(description.clone());
            debug!("Found description in old file: {}", description);
        }
    }

    debug!("Finished parsing old pyproject.toml");
    debug!("Found version: {:?}", poetry_version);
    debug!("Found description: {:?}", poetry_description);

    debug!("Reading new pyproject.toml at {}", pyproject_path.display());
    let content = fs::read_to_string(&pyproject_path)
        .map_err(|e| format!("Failed to read pyproject.toml: {}", e))?;

    let mut lines: Vec<String> = Vec::new();
    let mut in_project_section = false;
    let mut version_updated = false;
    let mut description_updated = false;

    debug!("Starting to update project metadata");
    for line in content.lines() {
        let trimmed = line.trim();
        debug!("Processing line: {}", trimmed);

        if trimmed == "[project]" {
            debug!("Found [project] section");
            in_project_section = true;
            lines.push(line.to_string());
            continue;
        } else if trimmed.starts_with('[') {
            if in_project_section {
                debug!("Leaving project section");
                if !version_updated && poetry_version.is_some() {
                    let version_line =
                        format!("version = \"{}\"", poetry_version.as_ref().unwrap());
                    debug!("Inserting version at section end: {}", version_line);
                    lines.push(version_line);
                    version_updated = true;
                }
                if !description_updated && poetry_description.is_some() {
                    let desc_line =
                        format!("description = \"{}\"", poetry_description.as_ref().unwrap());
                    debug!("Inserting description at section end: {}", desc_line);
                    lines.push(desc_line);
                    description_updated = true;
                }
            }
            in_project_section = false;
        }

        if !in_project_section {
            lines.push(line.to_string());
            continue;
        }

        // More robust metadata replacement
        if parse_toml_value(trimmed, "version").is_some() {
            if let Some(version) = poetry_version.as_ref() {
                debug!("Replacing version line with: {}", version);
                lines.push(format!("version = \"{}\"", version));
                version_updated = true;
                continue;
            }
        } else if parse_toml_value(trimmed, "description").is_some() {
            if let Some(description) = poetry_description.as_ref() {
                debug!("Replacing description line with: {}", description);
                lines.push(format!("description = \"{}\"", description));
                description_updated = true;
                continue;
            }
        }

        lines.push(line.to_string());
    }

    // Handle case where [project] section is the last section
    if in_project_section {
        debug!("Project section was the last section");
        if !version_updated && poetry_version.is_some() {
            let version_line = format!("version = \"{}\"", poetry_version.as_ref().unwrap());
            debug!("Inserting version at end: {}", version_line);
            lines.push(version_line);
            version_updated = true;
        }
        if !description_updated && poetry_description.is_some() {
            let desc_line = format!("description = \"{}\"", poetry_description.as_ref().unwrap());
            debug!("Inserting description at end: {}", desc_line);
            lines.push(desc_line);
            description_updated = true;
        }
    }

    // Log metadata update status
    if let Some(version) = poetry_version.as_ref() {
        if version_updated {
            info!("Successfully updated version to: {}", version);
        } else {
            warn!("Warning: Version was not updated in the new pyproject.toml");
        }
    }

    if let Some(description) = poetry_description.as_ref() {
        if description_updated {
            info!("Successfully updated description to: {}", description);
        } else {
            warn!("Warning: Description was not updated in the new pyproject.toml");
        }
    }

    debug!("Writing updated content back to pyproject.toml");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&pyproject_path)
        .map_err(|e| format!("Failed to open pyproject.toml for writing: {}", e))?;

    for line in lines {
        debug!("Writing line: {}", line);
        writeln!(file, "{}", line)
            .map_err(|e| format!("Failed to write line to pyproject.toml: {}", e))?;
    }

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
    } else {
        warn!("No metadata was found to update in the Poetry configuration");
    }

    debug!("Successfully completed pyproject.toml update");
    Ok(())
}

/// Helper function to parse TOML key-value pairs
fn parse_toml_value(line: &str, key: &str) -> Option<String> {
    // First check if the line contains our key
    if !line.contains(&format!("{} =", key)) && !line.contains(&format!("{}=", key)) {
        return None;
    }

    debug!("Attempting to parse {} from line: {}", key, line);

    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        debug!("Failed to split line on '='");
        return None;
    }

    let key_part = parts[0].trim();
    if key_part != key {
        debug!("Key mismatch: expected '{}', found '{}'", key, key_part);
        return None;
    }

    let value = parts[1].trim();
    // Handle both quoted and unquoted values
    let cleaned_value = value
        .trim_matches('"') // Remove double quotes
        .trim_matches('\'') // Remove single quotes
        .trim(); // Remove any remaining whitespace

    if cleaned_value.is_empty() {
        debug!("Found empty value for {}", key);
        None
    } else {
        debug!("Successfully parsed {} value: {}", key, cleaned_value);
        Some(cleaned_value.to_string())
    }
}
