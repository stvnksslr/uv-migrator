// src/utils/author.rs

use log::{debug, info};
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct Author {
    pub name: String,
    pub email: Option<String>,
}

impl Author {
    fn to_toml_string(&self) -> String {
        match &self.email {
            Some(email) => format!("{{ name = \"{}\", email = \"{}\" }}", self.name, email),
            None => format!("{{ name = \"{}\" }}", self.name),
        }
    }
}

pub fn extract_authors_from_setup_py(project_dir: &Path) -> Result<Vec<Author>, String> {
    let setup_py_path = project_dir.join("setup.py");
    if !setup_py_path.exists() {
        debug!("setup.py not found, skipping author extraction");
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&setup_py_path)
        .map_err(|e| format!("Failed to read setup.py: {}", e))?;

    let mut authors = Vec::new();

    // Extract author and author_email from setup() call
    if let Some(author) = extract_setup_param(&content, "author") {
        debug!("Found author: {}", author);
        let email = extract_setup_param(&content, "author_email");
        if let Some(ref email) = email {
            debug!("Found author email: {}", email);
        }
        authors.push(Author {
            name: author,
            email,
        });
    }

    if authors.is_empty() {
        debug!("No authors found in setup.py");
    } else {
        info!("Successfully extracted {} author(s) from setup.py", authors.len());
    }

    Ok(authors)
}

fn extract_setup_param(content: &str, param_name: &str) -> Option<String> {
    let param_pattern = format!("{}=\"", param_name);
    let param_pattern2 = format!("{}='", param_name);

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains(&param_pattern) || trimmed.contains(&param_pattern2) {
            // Find the value between quotes
            let start_idx = trimmed.find(&param_pattern)
                .or_else(|| trimmed.find(&param_pattern2))?;
            let quote_char = trimmed.chars().nth(start_idx + param_pattern.len() - 1)?;
            let value_start = start_idx + param_pattern.len();
            let value_end = trimmed[value_start..]
                .find(quote_char)
                .map(|i| value_start + i)?;
            
            let value = trimmed[value_start..value_end].to_string();
            debug!("Extracted {} value: {}", param_name, value);
            return Some(value);
        }
    }
    debug!("No {} found in content", param_name);
    None
}

pub fn update_authors(project_dir: &Path) -> Result<(), String> {
    debug!("Starting author update process");
    let pyproject_path = project_dir.join("pyproject.toml");
    let content = fs::read_to_string(&pyproject_path)
        .map_err(|e| format!("Failed to read pyproject.toml: {}", e))?;

    let authors = extract_authors_from_setup_py(project_dir)?;
    if authors.is_empty() {
        debug!("No authors to update");
        return Ok(());
    }

    let mut lines: Vec<String> = Vec::new();
    let mut section_lines: Vec<String> = Vec::new();
    let mut in_project_section = false;
    let mut skip_existing_authors = false;
    let mut authors_added = false;

    debug!("Processing pyproject.toml lines");
    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "[project]" {
            debug!("Found [project] section");
            in_project_section = true;
            section_lines.clear();
            lines.push(line.to_string());
            continue;
        } else if trimmed.starts_with('[') && trimmed != "[project]" {
            if in_project_section {
                // Insert any remaining section lines
                if !section_lines.is_empty() {
                    lines.extend(section_lines.drain(..));
                }
                // Add authors if we haven't yet and we're leaving the project section
                if !authors_added {
                    add_authors_section(&mut lines, &authors);
                    authors_added = true;
                }
            }
            in_project_section = false;
            lines.push(line.to_string());
            continue;
        }

        if in_project_section {
            if trimmed.starts_with("authors") {
                skip_existing_authors = true;
                continue;
            }
            if skip_existing_authors && (trimmed.starts_with('{') || trimmed.starts_with(']')) {
                continue;
            }
            if skip_existing_authors && !trimmed.is_empty() && !trimmed.starts_with('[') {
                skip_existing_authors = false;
            }

            // Check for readme line and insert authors after it
            if trimmed.starts_with("readme =") {
                section_lines.push(line.to_string());
                if !authors_added {
                    add_authors_section(&mut section_lines, &authors);
                    authors_added = true;
                }
                continue;
            }

            if !skip_existing_authors {
                section_lines.push(line.to_string());
            }
        } else {
            lines.push(line.to_string());
        }
    }

    // Handle case where [project] section is at the end
    if in_project_section {
        if !section_lines.is_empty() {
            lines.extend(section_lines.drain(..));
        }
        if !authors_added {
            add_authors_section(&mut lines, &authors);
        }
    }

    debug!("Writing updated content back to pyproject.toml");
    fs::write(&pyproject_path, lines.join("\n"))
        .map_err(|e| format!("Failed to write updated pyproject.toml: {}", e))?;

    info!("Successfully updated authors in pyproject.toml");
    Ok(())
}

fn add_authors_section(lines: &mut Vec<String>, authors: &[Author]) {
    lines.push("authors = [".to_string());
    for (i, author) in authors.iter().enumerate() {
        let mut line = "    ".to_string();
        line.push_str(&author.to_toml_string());
        if i < authors.len() - 1 {
            line.push(',');
        }
        lines.push(line);
    }
    lines.push("]".to_string());
}