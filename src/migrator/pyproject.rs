use std::path::Path;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write, Seek, SeekFrom, Read};
use log::info;

pub fn append_tool_sections(project_dir: &Path) -> Result<(), String> {
    let old_pyproject_path = project_dir.join("old.pyproject.toml");
    let new_pyproject_path = project_dir.join("pyproject.toml");

    if !old_pyproject_path.exists() {
        info!("old.pyproject.toml not found. This may indicate an issue with the migration process.");
        return Ok(());
    }

    let old_file = BufReader::new(fs::File::open(&old_pyproject_path)
        .map_err(|e| format!("Failed to open old.pyproject.toml: {}", e))?);

    let mut new_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&new_pyproject_path)
        .map_err(|e| format!("Failed to open new pyproject.toml for reading and writing: {}", e))?;

    let mut in_tool_section = false;
    let mut is_poetry_section = false;
    let mut current_section = String::new();
    let mut tool_sections = String::new();
    let mut existing_tool_sections = Vec::new();

    // Read the new file to check for existing [tool] sections
    let mut new_file_content = String::new();
    new_file.read_to_string(&mut new_file_content)
        .map_err(|e| format!("Failed to read new pyproject.toml: {}", e))?;

    for line in new_file_content.lines() {
        if line.starts_with("[tool.") && !line.starts_with("[tool.poetry") {
            existing_tool_sections.push(line.to_string());
        }
    }

    // Process the old file
    for line in old_file.lines() {
        let line = line.map_err(|e| format!("Error reading line: {}", e))?;

        if line.starts_with("[tool.") {
            if in_tool_section && !is_poetry_section
                && !existing_tool_sections.contains(&current_section.lines().next().unwrap_or("").to_string()) {
                tool_sections.push_str(&current_section);
            }
            in_tool_section = true;
            is_poetry_section = line.starts_with("[tool.poetry");
            current_section = String::new();
            if !is_poetry_section {
                current_section.push_str(&line);
                current_section.push('\n');
            }
        } else if line.starts_with('[') {
            if in_tool_section && !is_poetry_section
                && !existing_tool_sections.contains(&current_section.lines().next().unwrap_or("").to_string()) {
                tool_sections.push_str(&current_section);
            }
            in_tool_section = false;
            is_poetry_section = false;
            current_section.clear();
        } else if in_tool_section && !is_poetry_section {
            current_section.push_str(&line);
            current_section.push('\n');
        }
    }

    // Check the last section
    if in_tool_section && !is_poetry_section
        && !existing_tool_sections.contains(&current_section.lines().next().unwrap_or("").to_string()) {
        tool_sections.push_str(&current_section);
    }

    if !tool_sections.is_empty() {
        new_file.seek(SeekFrom::End(0)).map_err(|e| format!("Failed to seek to end of file: {}", e))?;
        writeln!(new_file).map_err(|e| format!("Failed to write newline: {}", e))?;
        write!(new_file, "{}", tool_sections.trim_start())
            .map_err(|e| format!("Failed to write tool sections: {}", e))?;
        info!("Appended [tool] sections to new pyproject.toml");
    } else {
        info!("No new [tool] sections found to append");
    }

    Ok(())
}