use crate::error::Result;
use crate::models::*;
use crate::utils::{
    author::extract_authors_from_poetry,
    author::extract_authors_from_setup_py,
    file_ops::FileTrackerGuard,
    parse_pip_conf, pyproject,
    toml::{read_toml, update_section, write_toml},
    update_pyproject_toml, update_url,
};
use log::info;
use std::path::Path;
use toml_edit::{Array, Formatted, Item, Value};

/// Merges all dependency groups into dev dependencies
pub fn merge_dependency_groups(dependencies: Vec<Dependency>) -> Vec<Dependency> {
    dependencies
        .into_iter()
        .map(|mut dep| {
            if matches!(dep.dep_type, DependencyType::Group(_)) {
                dep.dep_type = DependencyType::Dev;
            }
            dep
        })
        .collect()
}

/// Performs common migration tasks for all project types
pub fn perform_common_migrations(
    project_dir: &Path,
    file_tracker: &mut FileTrackerGuard,
    import_global_pip_conf: bool,
    additional_index_urls: &[String],
) -> Result<()> {
    let pyproject_path = project_dir.join("pyproject.toml");

    file_tracker.track_file(&pyproject_path)?;
    update_pyproject_toml(project_dir, &[])?;

    if let Some(version) = crate::utils::version::extract_version(project_dir)? {
        info!("Migrating version from setup.py");
        file_tracker.track_file(&pyproject_path)?;
        pyproject::update_project_version(project_dir, &version)?;
    }

    let mut extra_urls = Vec::new();
    if import_global_pip_conf {
        extra_urls.extend(parse_pip_conf()?);
    }

    // Explicitly add additional_index_urls to extra_urls
    if !additional_index_urls.is_empty() {
        info!("Adding custom index URLs: {:?}", additional_index_urls);
        extra_urls.extend(additional_index_urls.iter().cloned());
    }

    if !extra_urls.is_empty() {
        file_tracker.track_file(&pyproject_path)?;
        // Update pyproject.toml with extra URLs
        pyproject::update_uv_indices_from_urls(project_dir, &extra_urls)?;
    }

    info!("Migrating Tool sections");
    file_tracker.track_file(&pyproject_path)?;
    pyproject::append_tool_sections(project_dir)?;

    info!("Reordering pyproject.toml sections");
    file_tracker.track_file(&pyproject_path)?;
    crate::utils::toml::reorder_toml_sections(project_dir)?;

    Ok(())
}

/// Migrates Poetry-specific features
pub fn perform_poetry_migration(
    project_dir: &Path,
    file_tracker: &mut FileTrackerGuard,
) -> Result<()> {
    let pyproject_path = project_dir.join("pyproject.toml");

    info!("Checking for Poetry package sources to migrate");
    let sources = pyproject::extract_poetry_sources(project_dir)?;
    if !sources.is_empty() {
        file_tracker.track_file(&pyproject_path)?;
        pyproject::update_uv_indices(project_dir, &sources)?;
    }

    info!("Migrating Poetry authors");
    let poetry_authors = extract_authors_from_poetry(project_dir)?;
    if !poetry_authors.is_empty() {
        file_tracker.track_file(&pyproject_path)?;
        let mut doc = read_toml(&pyproject_path)?;
        let mut authors_array = Array::new();
        for author in &poetry_authors {
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
    }

    info!("Migrating Poetry scripts");
    file_tracker.track_file(&pyproject_path)?;
    pyproject::update_scripts(project_dir)?;

    info!("Checking Poetry build system");
    let mut doc = read_toml(&pyproject_path)?;
    if crate::utils::build_system::update_build_system(&mut doc, project_dir)? {
        info!("Migrated build system from Poetry to Hatchling");
        file_tracker.track_file(&pyproject_path)?;
        write_toml(&pyproject_path, &mut doc)?;
    }

    Ok(())
}

/// Migrates setup.py-specific features
pub fn perform_setup_py_migration(
    project_dir: &Path,
    file_tracker: &mut FileTrackerGuard,
) -> Result<()> {
    let pyproject_path = project_dir.join("pyproject.toml");

    info!("Migrating metadata from setup.py");
    let description =
        crate::migrators::setup_py::SetupPyMigrationSource::extract_description(project_dir)?;
    if let Some(desc) = description {
        file_tracker.track_file(&pyproject_path)?;
        pyproject::update_description(project_dir, &desc)?;
    }

    info!("Migrating URL from setup.py");
    let project_url = crate::migrators::setup_py::SetupPyMigrationSource::extract_url(project_dir)?;
    if let Some(url) = project_url {
        file_tracker.track_file(&pyproject_path)?;
        update_url(project_dir, &url)?;
    }

    info!("Migrating authors from setup.py");
    let setup_py_authors = extract_authors_from_setup_py(project_dir)?;
    if !setup_py_authors.is_empty() {
        file_tracker.track_file(&pyproject_path)?;
        let mut doc = read_toml(&pyproject_path)?;
        let mut authors_array = Array::new();
        for author in &setup_py_authors {
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
    }

    Ok(())
}

/// Migrates Pipenv-specific features
pub fn perform_pipenv_migration(
    project_dir: &Path,
    file_tracker: &mut FileTrackerGuard,
) -> Result<()> {
    let pyproject_path = project_dir.join("pyproject.toml");

    if let Ok(content) = std::fs::read_to_string(project_dir.join("Pipfile")) {
        if content.contains("[scripts]") {
            info!("Migrating Pipfile scripts");
            file_tracker.track_file(&pyproject_path)?;
        }
    }

    Ok(())
}

/// Migrates requirements.txt-specific features
pub fn perform_requirements_migration(
    project_dir: &Path,
    file_tracker: &mut FileTrackerGuard,
) -> Result<()> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let requirements_source = crate::migrators::requirements::RequirementsMigrationSource;
    let req_files = requirements_source.find_requirements_files(project_dir);

    for (file_path, _dep_type) in req_files {
        if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
            match file_name {
                "requirements.txt" | "requirements-dev.txt" => {
                    continue;
                }
                _ => {
                    if let Some(_group_name) = file_name
                        .strip_prefix("requirements-")
                        .and_then(|n| n.strip_suffix(".txt"))
                    {
                        info!("Configuring group from requirements file: {}", file_name);
                        file_tracker.track_file(&pyproject_path)?;
                    }
                }
            }
        }
    }

    Ok(())
}
