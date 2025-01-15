use std::{fs, path::Path};

use toml_edit::{DocumentMut, Item, Table};

/// Reads a TOML file and returns its content as a DocumentMut.
pub fn read_toml(path: &Path) -> Result<DocumentMut, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read TOML file '{}': {}", path.display(), e))?;

    content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse TOML in '{}': {}", path.display(), e))
}

/// Updates or creates a section in a TOML document.
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

/// Writes a TOML document to a file, removing any empty sections first.
pub fn write_toml(path: &Path, doc: &mut DocumentMut) -> Result<(), String> {
    cleanup_empty_sections(doc);
    fs::write(path, doc.to_string())
        .map_err(|e| format!("Failed to write TOML file '{}': {}", path.display(), e))
}

/// Removes empty sections from a TOML document recursively.
pub fn cleanup_empty_sections(doc: &mut DocumentMut) {
    let root_table = doc.as_table_mut();
    cleanup_table(root_table);
}

/// Recursively cleans up empty sections in a TOML table
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

/// Defines the expected order of fields within the [project] section
const PROJECT_FIELD_ORDER: &[&str] = &[
    "name",
    "version",
    "description",
    "authors",
    "readme",
    "requires-python",
    "dependencies",
    "classifiers",
    "optional-dependencies",
    "scripts",
    "urls",
];

/// Orders fields within a table according to a predefined order
fn order_table_fields(table: &mut Table, field_order: &[&str]) -> Table {
    let mut ordered = Table::new();
    ordered.set_implicit(table.is_implicit());

    // First add fields in the specified order
    for &field in field_order {
        if let Some(value) = table.remove(field) {
            ordered.insert(field, value);
        }
    }

    // Then add any remaining fields that weren't in the order list
    for (key, value) in table.iter() {
        if !field_order.contains(&key.to_string().as_str()) {
            ordered.insert(key, value.clone());
        }
    }

    ordered
}

/// Updates the reorder_toml_sections function to include field ordering
pub fn reorder_toml_sections(project_dir: &Path) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let content = fs::read_to_string(&pyproject_path)
        .map_err(|e| format!("Failed to read pyproject.toml: {}", e))?;

    let mut doc = content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse TOML: {}", e))?;

    // Order the [project] section fields if it exists
    if let Some(Item::Table(project_table)) = doc.get_mut("project") {
        let ordered_project = order_table_fields(project_table, PROJECT_FIELD_ORDER);
        doc.insert("project", Item::Table(ordered_project));
    }

    // Continue with existing section ordering logic
    let mut sections: Vec<(String, Item)> = Vec::new();
    let mut tool_sections: Vec<(String, Item)> = Vec::new();

    // Collect and categorize sections
    for (key, value) in doc.iter() {
        let owned_key = key.to_string();
        let owned_value = value.clone();

        if owned_key.starts_with("tool.") {
            tool_sections.push((owned_key, owned_value));
        } else {
            sections.push((owned_key, owned_value));
        }
    }

    // Clear the document
    let keys_to_remove: Vec<String> = doc.as_table().iter().map(|(k, _)| k.to_string()).collect();
    for key in keys_to_remove {
        doc.remove(&key);
    }

    // Write back sections in the desired order
    let section_order = ["project", "build-system"];

    // First, add ordered known sections
    for &section_name in section_order.iter() {
        if let Some((_, item)) = sections.iter().find(|(key, _)| key == section_name) {
            doc.insert(section_name, item.clone());
        }
    }

    // Then add any remaining non-tool sections that weren't in the order list
    for (key, item) in sections.iter() {
        if !section_order.contains(&key.as_str()) {
            doc.insert(key, item.clone());
        }
    }

    // Finally add tool sections
    for (key, item) in tool_sections {
        doc.insert(&key, item);
    }

    // Write the reordered content back to the file
    fs::write(&pyproject_path, doc.to_string())
        .map_err(|e| format!("Failed to write pyproject.toml: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_project_field_ordering() {
        let temp_dir = TempDir::new().unwrap();
        let input_content = r#"[project]
dependencies = ["package1>=1.0.0"]
name = "test-project"
version = "1.0.0"
authors = [{ name = "Test Author", email = "test@example.com" }]
description = "Test description"
"#;
        fs::write(temp_dir.path().join("pyproject.toml"), input_content).unwrap();

        reorder_toml_sections(temp_dir.path()).unwrap();

        let result = fs::read_to_string(temp_dir.path().join("pyproject.toml")).unwrap();

        // Verify field order
        let name_pos = result.find("name").unwrap();
        let version_pos = result.find("version").unwrap();
        let description_pos = result.find("description").unwrap();
        let authors_pos = result.find("authors").unwrap();
        let dependencies_pos = result.find("dependencies").unwrap();

        assert!(name_pos < version_pos, "name should come before version");
        assert!(
            version_pos < description_pos,
            "version should come before description"
        );
        assert!(
            description_pos < authors_pos,
            "description should come before authors"
        );
        assert!(
            authors_pos < dependencies_pos,
            "authors should come before dependencies"
        );
    }
}
