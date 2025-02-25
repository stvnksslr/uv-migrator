use crate::error::{Error, Result};
use crate::migrators::setup_py::SetupPyMigrationSource;
use std::path::Path;
use toml_edit::DocumentMut;

#[derive(Debug)]
pub struct Author {
    pub name: String,
    pub email: Option<String>,
}

pub fn extract_authors_from_setup_py(project_dir: &Path) -> Result<Vec<Author>> {
    let setup_py_path = project_dir.join("setup.py");
    if !setup_py_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&setup_py_path).map_err(|e| Error::FileOperation {
        path: setup_py_path.clone(),
        message: format!("Failed to read setup.py: {}", e),
    })?;

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

pub fn extract_authors_from_poetry(project_dir: &Path) -> Result<Vec<Author>> {
    let old_pyproject_path = project_dir.join("old.pyproject.toml");
    if !old_pyproject_path.exists() {
        return Ok(vec![]);
    }

    let content =
        std::fs::read_to_string(&old_pyproject_path).map_err(|e| Error::FileOperation {
            path: old_pyproject_path.clone(),
            message: format!("Failed to read old.pyproject.toml: {}", e),
        })?;

    let doc = content.parse::<DocumentMut>().map_err(Error::Toml)?;

    // Extract authors from project section (Poetry 2.0 style)
    if let Some(project) = doc.get("project") {
        if let Some(authors_array) = project.get("authors").and_then(|a| a.as_array()) {
            let mut results = Vec::new();
            for author_value in authors_array.iter() {
                if let Some(author_str) = author_value.as_str() {
                    results.push(parse_author_string(author_str));
                } else if let Some(author_table) = author_value.as_inline_table() {
                    // Poetry 2.0 style inline table
                    let name = author_table
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    let email = author_table
                        .get("email")
                        .and_then(|e| e.as_str())
                        .map(|s| s.to_string());

                    results.push(Author { name, email });
                }
            }
            return Ok(results);
        }
    }

    // Fallback to traditional Poetry section
    let authors = match doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|poetry| poetry.get("authors"))
    {
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

    // First, check for Poetry 2.0 style inline table author format
    if author_str.starts_with('{') && author_str.ends_with('}') {
        // Remove {} and split by commas
        let content = &author_str[1..author_str.len() - 1];
        let mut name = String::new();
        let mut email = None;

        for part in content.split(',') {
            let part = part.trim();
            if let Some(name_part) = part
                .strip_prefix("name = ")
                .or_else(|| part.strip_prefix("name="))
            {
                name = name_part.trim_matches(&['"', '\''][..]).to_string();
            }
            if let Some(email_part) = part
                .strip_prefix("email = ")
                .or_else(|| part.strip_prefix("email="))
            {
                email = Some(email_part.trim_matches(&['"', '\''][..]).to_string());
            }
        }

        return Author { name, email };
    }

    // Classic Poetry author format with email in angle brackets
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
