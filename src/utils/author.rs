use crate::migrators::setup_py::SetupPyMigrationSource;
use crate::utils::toml::{read_toml, update_section, write_toml};
use std::path::Path;
use toml_edit::{Array, Formatted, Item, Value};

#[derive(Debug)]
pub struct Author {
    pub name: String,
    pub email: Option<String>,
}

pub fn extract_authors_from_setup_py(project_dir: &Path) -> Result<Vec<Author>, String> {
    let setup_py_path = project_dir.join("setup.py");
    if !setup_py_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&setup_py_path)
        .map_err(|e| format!("Failed to read setup.py: {}", e))?;

    let mut authors = Vec::new();

    // Extract author and author_email from setup()
    if let Some(start_idx) = content.find("setup(") {
        let bracket_content = SetupPyMigrationSource::extract_setup_content(&content[start_idx..])?;

        if let Some(name) = SetupPyMigrationSource::extract_parameter(&bracket_content, "author") {
            let email = SetupPyMigrationSource::extract_parameter(&bracket_content, "author_email");
            authors.push(Author { name, email });
        }
    }

    Ok(authors)
}

pub fn update_authors(project_dir: &Path) -> Result<(), String> {
    let authors = extract_authors_from_setup_py(project_dir)?;
    if authors.is_empty() {
        return Ok(());
    }

    let pyproject_path = project_dir.join("pyproject.toml");
    let mut doc = read_toml(&pyproject_path)?;

    let mut authors_array = Array::new();
    for author in &authors {
        let mut table = toml_edit::InlineTable::new();
        table.insert("name", Value::String(Formatted::new(author.name.clone())));
        if let Some(ref email) = author.email {
            table.insert("email", Value::String(Formatted::new(email.clone())));
        }
        authors_array.push(Value::InlineTable(table));
    }

    update_section(
        &mut doc,
        &["project", "authors"],
        Item::Value(Value::Array(authors_array)),
    );

    write_toml(&pyproject_path, &mut doc)?;
    Ok(())
}
