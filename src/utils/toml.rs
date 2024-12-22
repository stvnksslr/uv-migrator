use std::fs;
use std::path::Path;
use toml_edit::{DocumentMut, Item, Table};

/// Reads a TOML file and returns its content as a DocumentMut.
pub fn read_toml(path: &Path) -> Result<DocumentMut, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read TOML file '{}': {}", path.display(), e))?;

    content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse TOML in '{}': {}", path.display(), e))
}

/// Removes empty sections from a TOML document
pub fn cleanup_empty_sections(doc: &mut DocumentMut) {
    // Create a list of empty section keys to remove
    let empty_sections: Vec<String> = doc
        .as_table()
        .iter()
        .filter(|(_, value)| match value.as_table() {
            Some(table) => table.is_empty(),
            None => false,
        })
        .map(|(key, _)| key.to_string())
        .collect();

    // Remove each empty section
    for key in empty_sections {
        doc.remove(&key);
    }
}

/// Writes a TOML document to a file, removing any empty sections first.
pub fn write_toml(path: &Path, doc: &mut DocumentMut) -> Result<(), String> {
    cleanup_empty_sections(doc);
    fs::write(path, doc.to_string())
        .map_err(|e| format!("Failed to write TOML file '{}': {}", path.display(), e))
}

/// Updates or creates a section in a TOML document.
pub fn update_section(doc: &mut DocumentMut, section_path: &[&str], content: Item) {
    let mut current = doc.as_table_mut();

    for &section in &section_path[..section_path.len() - 1] {
        if !current.contains_key(section) {
            current.insert(section, Item::Table(Table::new()));
        }
        current = current[section].as_table_mut().unwrap();
    }

    let last_section = section_path.last().unwrap();
    current.insert(last_section, content);
}
