use log::info;
use std::path::Path;
use toml_edit::{Array, Formatted, Item, Table, Value};

use crate::utils::toml::{read_toml, update_section, write_toml};

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
        let bracket_content = extract_setup_content(&content[start_idx..])?;

        if let Some(name) = extract_parameter(&bracket_content, "author") {
            let email = extract_parameter(&bracket_content, "author_email");
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
        let mut author_table = Table::new();
        author_table.insert(
            "name",
            Item::Value(Value::String(Formatted::new(author.name.clone()))),
        );
        if let Some(ref email) = author.email {
            author_table.insert(
                "email",
                Item::Value(Value::String(Formatted::new(email.clone()))),
            );
        }
        authors_array.push(Value::from_iter(vec![(
            "name",
            Value::String(Formatted::new(author.name.clone())),
        )]));
        if let Some(ref email) = author.email {
            if let Some(last) = authors_array.get_mut(authors_array.len() - 1) {
                if let Some(table) = last.as_inline_table_mut() {
                    table.insert("email", Value::String(Formatted::new(email.clone())));
                }
            }
        }
    }

    update_section(
        &mut doc,
        &["project", "authors"],
        Item::Value(Value::Array(authors_array)),
    );

    let mut doc = read_toml(&pyproject_path)?;
    write_toml(&pyproject_path, &mut doc)?;
    info!("Successfully updated authors in pyproject.toml");
    Ok(())
}

fn extract_setup_content(content: &str) -> Result<String, String> {
    let lines = content.lines().enumerate().peekable();
    let mut setup_content = String::new();
    let mut in_setup = false;
    let mut paren_count = 0;

    for (_, line) in lines {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if !in_setup {
            if trimmed.starts_with("setup(") {
                in_setup = true;
                paren_count = 1;
                // Extract everything after setup(
                if let Some(content) = line.split("setup(").nth(1) {
                    setup_content.push_str(content);
                    setup_content.push('\n');
                }
            }
        } else {
            // Count parentheses in the line
            for c in line.chars() {
                match c {
                    '(' => paren_count += 1,
                    ')' => paren_count -= 1,
                    _ => {}
                }
            }

            setup_content.push_str(line);
            setup_content.push('\n');

            if paren_count == 0 {
                break;
            }
        }
    }

    if !in_setup {
        return Err("Could not find setup() call".to_string());
    }
    if paren_count > 0 {
        return Err("Could not find matching closing parenthesis for setup()".to_string());
    }

    Ok(setup_content)
}

fn extract_parameter(content: &str, param_name: &str) -> Option<String> {
    let param_pattern = format!("{} = ", param_name);
    let param_pattern2 = format!("{}=", param_name);

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with(&param_pattern) || trimmed.starts_with(&param_pattern2) {
            // Handle direct string assignment
            if trimmed.contains('"') || trimmed.contains('\'') {
                return extract_string_value(trimmed);
            }
            // Handle variable assignment
            if let Some(value) = trimmed.split('=').nth(1) {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn extract_string_value(line: &str) -> Option<String> {
    let after_equals = line.split('=').nth(1)?.trim();

    // Handle different quote types
    let (quote_char, content) = match after_equals.chars().next()? {
        '\'' => ('\'', &after_equals[1..]),
        '"' => ('"', &after_equals[1..]),
        _ => return None,
    };

    // Find matching end quote
    let end_pos = content.find(quote_char)?;
    let value = content[..end_pos].to_string();

    Some(value)
}
