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
