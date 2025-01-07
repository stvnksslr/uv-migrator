use crate::migrators::setup_py::SetupPyMigrationSource;
use crate::utils::toml::{read_toml, update_section, write_toml};
use std::path::Path;
use toml_edit::{Array, DocumentMut, Formatted, Item, Value};

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

pub fn extract_authors_from_poetry(project_dir: &Path) -> Result<Vec<Author>, String> {
    let old_pyproject_path = project_dir.join("old.pyproject.toml");
    if !old_pyproject_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&old_pyproject_path)
        .map_err(|e| format!("Failed to read old.pyproject.toml: {}", e))?;

    let doc = content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse TOML: {}", e))?;

    // Get poetry section from the TOML document
    let poetry = doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .ok_or_else(|| "No [tool.poetry] section found".to_string())?;

    // Get authors array
    let authors = match poetry.get("authors") {
        Some(array) => {
            let mut result = Vec::new();
            if let Some(arr) = array.as_array() {
                for value in arr.iter() {
                    if let Some(author_str) = value.as_str() {
                        result.push(parse_author_string(author_str));
                    }
                }
            }
            result
        }
        None => vec![],
    };

    Ok(authors)
}

fn parse_author_string(author_str: &str) -> Author {
    let author_str = author_str.trim();

    // Pattern match for email between angle brackets
    let (name, email) = match (author_str.rfind('<'), author_str.rfind('>')) {
        (Some(start), Some(end)) if start < end => {
            let name = author_str[..start].trim().to_string();
            let email = author_str[start + 1..end].trim().to_string();
            (name, Some(email))
        }
        _ => (author_str.to_string(), None),
    };

    Author { name, email }
}

pub fn update_authors(project_dir: &Path) -> Result<(), String> {
    let mut authors = extract_authors_from_poetry(project_dir)?;
    if authors.is_empty() {
        authors = extract_authors_from_setup_py(project_dir)?;
    }

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
