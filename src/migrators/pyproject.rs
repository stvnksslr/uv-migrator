use log::{debug, info};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;

pub fn append_tool_sections(project_dir: &Path) -> Result<(), String> {
    let old_pyproject_path = project_dir.join("old.pyproject.toml");
    let new_pyproject_path = project_dir.join("pyproject.toml");

    if !old_pyproject_path.exists() {
        debug!(
            "old.pyproject.toml not found. This may indicate an issue with the migration process."
        );
        return Ok(());
    }

    debug!(
        "Reading old pyproject.toml from: {}",
        old_pyproject_path.display()
    );
    let old_file = BufReader::new(
        fs::File::open(&old_pyproject_path)
            .map_err(|e| format!("Failed to open old.pyproject.toml: {}", e))?,
    );

    debug!(
        "Opening new pyproject.toml for reading and writing: {}",
        new_pyproject_path.display()
    );
    let mut new_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&new_pyproject_path)
        .map_err(|e| {
            format!(
                "Failed to open new pyproject.toml for reading and writing: {}",
                e
            )
        })?;

    let mut in_tool_section = false;
    let mut is_poetry_section = false;
    let mut current_section = String::new();
    let mut tool_sections = String::new();
    let mut existing_tool_sections = Vec::new();

    // Read existing tool sections from new file
    let mut new_file_content = String::new();
    new_file
        .read_to_string(&mut new_file_content)
        .map_err(|e| format!("Failed to read new pyproject.toml: {}", e))?;

    for line in new_file_content.lines() {
        if line.starts_with("[tool.") && !line.starts_with("[tool.poetry") {
            existing_tool_sections.push(line.to_string());
        }
    }

    debug!("Existing tool sections found: {:?}", existing_tool_sections);

    // Process the old file to extract tool sections
    for line in old_file.lines() {
        let line = line.map_err(|e| format!("Error reading line: {}", e))?;

        if line.starts_with("[tool.") {
            debug!("Found tool section: {}", line);
            if in_tool_section
                && !is_poetry_section
                && !existing_tool_sections
                    .contains(&current_section.lines().next().unwrap_or("").to_string())
            {
                debug!("Adding previous tool section to output");
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
            if in_tool_section
                && !is_poetry_section
                && !existing_tool_sections
                    .contains(&current_section.lines().next().unwrap_or("").to_string())
            {
                debug!("Adding final tool section before new section");
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

    // Handle the last section
    if in_tool_section
        && !is_poetry_section
        && !existing_tool_sections
            .contains(&current_section.lines().next().unwrap_or("").to_string())
    {
        debug!("Adding final tool section");
        tool_sections.push_str(&current_section);
    }

    // Append the tool sections to the new file
    if !tool_sections.is_empty() {
        debug!("Appending tool sections to new pyproject.toml");
        new_file
            .seek(SeekFrom::End(0))
            .map_err(|e| format!("Failed to seek to end of file: {}", e))?;
        writeln!(new_file).map_err(|e| format!("Failed to write newline: {}", e))?;
        write!(new_file, "{}", tool_sections.trim_start())
            .map_err(|e| format!("Failed to write tool sections: {}", e))?;
        info!("Appended [tool] sections to new pyproject.toml");
    } else {
        debug!("No new [tool] sections to append");
    }

    Ok(())
}
