use std::fs;
use std::path::Path;
use toml_edit::{DocumentMut, Item, Table};

/// Reads a TOML file and returns its content as a DocumentMut.
///
/// # Arguments
///
/// * `path` - The path to the TOML file to read
///
/// # Returns
///
/// * `Result<DocumentMut, String>` - The parsed TOML document or an error message
///
/// # Errors
///
/// Returns an error if:
/// * The file cannot be read
/// * The file content cannot be parsed as valid TOML
pub fn read_toml(path: &Path) -> Result<DocumentMut, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read TOML file '{}': {}", path.display(), e))?;

    content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse TOML in '{}': {}", path.display(), e))
}

/// Removes empty sections from a TOML document recursively.
///
/// # Arguments
///
/// * `doc` - The TOML document to clean up
///
/// This function traverses the document tree and removes:
/// * Empty tables
/// * Tables that only contain empty nested tables
/// * Empty arrays
pub fn cleanup_empty_sections(doc: &mut DocumentMut) {
    let root_table = doc.as_table_mut();
    cleanup_table(root_table);
}

/// Recursively cleans up empty sections in a TOML table
///
/// # Arguments
///
/// * `table` - The table to clean up
///
/// This function uses a three-pass approach:
/// 1. Collect keys that need checking (tables and arrays)
/// 2. Clean up nested tables recursively
/// 3. Remove empty sections
fn cleanup_table(table: &mut Table) {
    // First pass: Collect keys to clean up
    let keys_to_check: Vec<String> = table
        .iter()
        .filter(|(_, value)| value.is_table() || value.is_array())
        .map(|(key, _)| key.to_string())
        .collect();

    // Second pass: Clean up nested tables
    for key in &keys_to_check {
        if let Some(value) = table.get_mut(key) {
            if let Some(nested_table) = value.as_table_mut() {
                cleanup_table(nested_table);
            }
        }
    }

    // Third pass: Remove empty tables and sections
    let keys_to_remove: Vec<String> = table
        .iter()
        .filter(|(_, value)| is_empty_section(value))
        .map(|(key, _)| key.to_string())
        .collect();

    for key in keys_to_remove {
        table.remove(&key);
    }
}

/// Checks if a TOML item is empty
///
/// # Arguments
///
/// * `item` - The TOML item to check
///
/// # Returns
///
/// * `bool` - true if the item is considered empty
///
/// An item is considered empty if it is:
/// * An empty table
/// * A table containing only empty tables
/// * An empty array
/// * An empty array of tables
/// * A None value
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

/// Writes a TOML document to a file, removing any empty sections first.
///
/// # Arguments
///
/// * `path` - The path where to write the TOML file
/// * `doc` - The document to write
///
/// # Returns
///
/// * `Result<(), String>` - Ok(()) on success, or error message on failure
///
/// # Errors
///
/// Returns an error if:
/// * The file cannot be written
/// * The parent directory doesn't exist
pub fn write_toml(path: &Path, doc: &mut DocumentMut) -> Result<(), String> {
    cleanup_empty_sections(doc);
    fs::write(path, doc.to_string())
        .map_err(|e| format!("Failed to write TOML file '{}': {}", path.display(), e))
}

/// Updates or creates a section in a TOML document.
///
/// # Arguments
///
/// * `doc` - The document to update
/// * `section_path` - Array of section names forming the path to the target section
/// * `content` - The content to insert at the specified path
///
/// # Example
///
/// ```ignore
/// update_section(&mut doc, &["tool", "black"], Item::from("line-length = 88"));
/// ```
///
/// This will create or update the [tool.black] section.
pub fn update_section(doc: &mut DocumentMut, section_path: &[&str], content: Item) {
    let mut current = doc.as_table_mut();

    for &section in &section_path[..section_path.len() - 1] {
        if !current.contains_key(section) {
            let mut new_table = Table::new();
            new_table.set_implicit(true);
            current.insert(section, Item::Table(new_table));
        }
        current = current[section].as_table_mut().unwrap();
    }

    let last_section = section_path.last().unwrap();
    current.insert(last_section, content);
}

/// Reorders sections in a TOML file to group all [tool.*] sections at the bottom
/// while preserving the exact content and formatting of each section.
///
/// The desired order is:
/// 1. [project] and related sections
/// 2. [build-system]
/// 3. Other non-tool sections
/// 4. All [tool.*] sections
///
/// This should be called as the final step in the migration process after all other TOML
/// modifications are complete.
///
/// # Arguments
///
/// * `project_dir` - The directory containing the pyproject.toml file
///
/// # Returns
///
/// * `Result<(), String>` - Ok(()) on success, or error message on failure
pub fn reorder_toml_sections(project_dir: &Path) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let content = fs::read_to_string(&pyproject_path)
        .map_err(|e| format!("Failed to read pyproject.toml: {}", e))?;

    // Split the content into sections while preserving empty lines between sections
    let mut sections: Vec<String> = Vec::new();
    let mut current_section = String::new();
    let mut in_section = false;

    for line in content.lines() {
        if line.starts_with('[') {
            if in_section {
                sections.push(current_section.trim_end().to_string());
                current_section = String::new();
            }
            in_section = true;
            current_section.push_str(line);
            current_section.push('\n');
        } else if !in_section && !line.trim().is_empty() {
            // If we find content before any section, treat it as its own section
            sections.push(line.to_string());
        } else if in_section {
            current_section.push_str(line);
            current_section.push('\n');
        }
    }

    if !current_section.is_empty() {
        sections.push(current_section.trim_end().to_string());
    }

    // Sort sections into categories
    let mut project_sections: Vec<String> = Vec::new();
    let mut build_system_section: Option<String> = None;
    let mut tool_sections: Vec<String> = Vec::new();
    let mut other_sections: Vec<String> = Vec::new();

    for section in sections {
        if section.trim().starts_with("[project") {
            project_sections.push(section);
        } else if section.trim().starts_with("[build-system]") {
            build_system_section = Some(section);
        } else if section.trim().starts_with("[tool.") {
            tool_sections.push(section);
        } else {
            other_sections.push(section);
        }
    }

    // Combine sections in the desired order
    let mut final_content = String::new();

    // Add project sections first
    for section in project_sections {
        final_content.push_str(&section);
        final_content.push_str("\n\n");
    }

    // Add build-system section if it exists
    if let Some(build_section) = build_system_section {
        final_content.push_str(&build_section);
        final_content.push_str("\n\n");
    }

    // Add other non-tool sections
    for section in other_sections {
        final_content.push_str(&section);
        final_content.push_str("\n\n");
    }

    // Add tool sections last
    for section in tool_sections {
        final_content.push_str(&section);
        final_content.push_str("\n\n");
    }

    // Remove extra newlines at the end while preserving one final newline
    let final_content = final_content.trim_end().to_string() + "\n";

    // Write the reordered content back to the file
    fs::write(&pyproject_path, final_content)
        .map_err(|e| format!("Failed to write pyproject.toml: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_reorder_toml_sections() {
        let temp_dir = TempDir::new().unwrap();
        let input_content = r#"[tool.black]
line-length = 120

[project]
name = "test-project"
version = "1.0.0"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.ruff]
line-length = 120

[other-section]
key = "value"
"#;

        fs::write(temp_dir.path().join("pyproject.toml"), input_content).unwrap();

        reorder_toml_sections(temp_dir.path()).unwrap();

        let result = fs::read_to_string(temp_dir.path().join("pyproject.toml")).unwrap();

        // Verify order of sections
        let project_pos = result.find("[project]").unwrap();
        let build_system_pos = result.find("[build-system]").unwrap();
        let other_section_pos = result.find("[other-section]").unwrap();
        let tool_black_pos = result.find("[tool.black]").unwrap();
        let tool_ruff_pos = result.find("[tool.ruff]").unwrap();

        assert!(
            project_pos < build_system_pos,
            "project should come before build-system"
        );
        assert!(
            build_system_pos < other_section_pos,
            "build-system should come before other sections"
        );
        assert!(
            other_section_pos < tool_black_pos,
            "other sections should come before tool sections"
        );
        assert!(
            tool_black_pos < tool_ruff_pos,
            "tool sections should be grouped together"
        );
    }

    #[test]
    fn test_preserve_formatting() {
        let temp_dir = TempDir::new().unwrap();
        let input_content = r#"[tool.black]
line-length = 120  # Comment
target-version = [  # Multi-line
    "py39",         # With alignment
    "py310"        # And comments
]

[project]
name = "test"  # Project name
"#;

        fs::write(temp_dir.path().join("pyproject.toml"), input_content).unwrap();

        reorder_toml_sections(temp_dir.path()).unwrap();

        let result = fs::read_to_string(temp_dir.path().join("pyproject.toml")).unwrap();

        // Verify comments and formatting are preserved
        assert!(result.contains("line-length = 120  # Comment"));
        assert!(result.contains(
            r#"target-version = [  # Multi-line
    "py39",         # With alignment
    "py310"        # And comments
]"#
        ));
        assert!(result.contains(r#"name = "test"  # Project name"#));
    }

    #[test]
    fn test_empty_lines_handling() {
        let temp_dir = TempDir::new().unwrap();
        let input_content = r#"[tool.black]
line-length = 120

# Comment between sections

[project]
name = "test"

"#;

        fs::write(temp_dir.path().join("pyproject.toml"), input_content).unwrap();

        reorder_toml_sections(temp_dir.path()).unwrap();

        let result = fs::read_to_string(temp_dir.path().join("pyproject.toml")).unwrap();

        // Verify empty lines are preserved within sections
        assert!(result.contains("\n\n"));
        assert!(result.ends_with('\n'));
    }
}
