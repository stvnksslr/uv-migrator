use crate::migrators::detect::{PoetryProjectType, ProjectType};
use crate::utils::build_system::update_build_system;
use crate::utils::{
    author::extract_authors_from_poetry,
    author::extract_authors_from_setup_py,
    parse_pip_conf, pyproject,
    toml::{read_toml, update_section, write_toml},
    update_pyproject_toml, update_url, FileTrackerGuard,
};
use log::info;
use poetry::PoetryMigrationSource;
use setup_py::SetupPyMigrationSource;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use toml_edit::{Array, Formatted, Item, Value};

mod dependency;
mod detect;
pub mod pipenv;
pub mod poetry;
pub mod requirements;
pub mod setup_py;

pub use dependency::{Dependency, DependencyType};
pub use detect::detect_project_type;

pub trait MigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>, String>;
}

pub trait MigrationTool {
    fn prepare_project(
        &self,
        project_dir: &Path,
        file_tracker: &mut FileTrackerGuard,
        project_type: &ProjectType,
    ) -> Result<(), String>;

    fn add_dependencies(
        &self,
        project_dir: &Path,
        dependencies: &[Dependency],
    ) -> Result<(), String>;
}

pub struct UvTool;

impl MigrationTool for UvTool {
    fn prepare_project(
        &self,
        project_dir: &Path,
        file_tracker: &mut FileTrackerGuard,
        project_type: &ProjectType,
    ) -> Result<(), String> {
        let pyproject_path = project_dir.join("pyproject.toml");
        let backup_path = project_dir.join("old.pyproject.toml");

        // Backup existing pyproject.toml if it exists
        if pyproject_path.exists() {
            file_tracker.track_rename(&pyproject_path, &backup_path)?;
            fs::rename(&pyproject_path, &backup_path)
                .map_err(|e| format!("Failed to rename existing pyproject.toml: {}", e))?;
            info!("Renamed existing pyproject.toml to old.pyproject.toml");
        }
        file_tracker.track_file(&pyproject_path)?;

        // Determine if this is a package project
        let is_package = matches!(
            project_type,
            &ProjectType::Poetry(PoetryProjectType::Package) | &ProjectType::SetupPy
        );

        // Extract Python version for Poetry projects
        let python_version = match project_type {
            ProjectType::Poetry(_) => {
                match PoetryMigrationSource::extract_python_version(project_dir)? {
                    Some(version) => {
                        info!("Found Python version constraint: {}", version);
                        Some(version)
                    }
                    None => {
                        info!("No Python version constraint found, using --no-pin-python");
                        None
                    }
                }
            }
            _ => None,
        };

        // Find uv executable
        let uv_path =
            which::which("uv").map_err(|e| format!("Failed to find uv command: {}", e))?;

        // Build uv init command
        let mut command = std::process::Command::new(&uv_path);
        command.arg("init");

        // Add appropriate flags based on project configuration
        if python_version.is_none() {
            command.arg("--no-pin-python");
        }

        if is_package {
            command.arg("--package");
        }

        if let Some(version) = python_version {
            command.arg("--python").arg(version);
        }

        // Set working directory and execute command
        command.current_dir(project_dir);

        info!(
            "Executing uv init command: {:?}",
            command.get_args().collect::<Vec<_>>()
        );

        let output = command
            .output()
            .map_err(|e| format!("Failed to execute uv init: {}", e))?;

        if output.status.success() {
            info!("Successfully initialized new project with uv init");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("uv init failed: {}", stderr))
        }
    }

    fn add_dependencies(
        &self,
        project_dir: &Path,
        dependencies: &[Dependency],
    ) -> Result<(), String> {
        let uv_path =
            which::which("uv").map_err(|e| format!("Failed to find uv command: {}", e))?;

        let mut grouped_deps: HashMap<&DependencyType, Vec<&Dependency>> = HashMap::new();
        for dep in dependencies {
            grouped_deps.entry(&dep.dep_type).or_default().push(dep);
        }

        for (dep_type, deps) in grouped_deps {
            if deps.is_empty() {
                continue;
            }

            let mut command = std::process::Command::new(&uv_path);
            command.arg("add");

            match dep_type {
                DependencyType::Dev => {
                    command.arg("--dev");
                }
                DependencyType::Group(group_name) => {
                    command.arg("--group").arg(group_name);
                }
                DependencyType::Main => {}
            }

            command.current_dir(project_dir);

            for dep in deps {
                let mut dep_str = if let Some(version) = &dep.version {
                    let version = version.trim();
                    if version.contains(',') || version.starts_with("~=") {
                        format!("{}{}", dep.name, version)
                    } else if let Some(stripped) = version.strip_prefix('~') {
                        format!("{}~={}", dep.name, stripped)
                    } else if let Some(stripped) = version.strip_prefix('^') {
                        format!("{}>={}", dep.name, stripped)
                    } else if version.starts_with(['>', '<', '=']) {
                        format!("{}{}", dep.name, version)
                    } else {
                        format!("{}=={}", dep.name, version)
                    }
                } else {
                    dep.name.clone()
                };

                if let Some(markers) = &dep.environment_markers {
                    dep_str.push_str(&format!("; {}", markers));
                }
                command.arg(dep_str);
            }

            info!(
                "Running uv add command for {:?} dependencies: {:?}",
                dep_type, command
            );

            let output = command
                .output()
                .map_err(|e| format!("Failed to execute uv command: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!(
                    "Failed to add {:?} dependencies: {}",
                    dep_type, stderr
                ));
            }
        }

        info!("All dependencies added successfully!");
        Ok(())
    }
}

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

pub fn run_migration(
    project_dir: &Path,
    import_global_pip_conf: bool,
    additional_index_urls: &[String],
    merge_groups: bool,
) -> Result<(), String> {
    let mut file_tracker = FileTrackerGuard::new();
    let hello_py_path = project_dir.join("hello.py");
    let pyproject_path = project_dir.join("pyproject.toml");
    let old_pyproject_path = project_dir.join("old.pyproject.toml");

    if hello_py_path.exists() {
        file_tracker.track_file(&hello_py_path)?;
    }

    let result = (|| {
        let project_type: ProjectType = detect_project_type(project_dir)?;
        info!("Detected project type: {:?}", project_type);

        // Extract dependencies based on project type
        let migration_source: Box<dyn MigrationSource> = match project_type {
            ProjectType::Poetry(_) => Box::new(poetry::PoetryMigrationSource),
            ProjectType::Pipenv => Box::new(pipenv::PipenvMigrationSource),
            ProjectType::Requirements => Box::new(requirements::RequirementsMigrationSource),
            ProjectType::SetupPy => Box::new(SetupPyMigrationSource),
        };

        let mut dependencies = migration_source.extract_dependencies(project_dir)?;
        info!("Extracted {} dependencies", dependencies.len());

        if merge_groups {
            dependencies = merge_dependency_groups(dependencies);
            info!("Merged all dependency groups into dev dependencies");
        }

        // Initialize UV project
        let migration_tool = UvTool;
        migration_tool.prepare_project(project_dir, &mut file_tracker, &project_type)?;
        info!("Project initialized with UV");

        // Add dependencies
        migration_tool.add_dependencies(project_dir, &dependencies)?;
        info!("Dependencies added successfully");

        // Track pyproject.toml for potential updates
        file_tracker.track_file(&pyproject_path)?;

        if old_pyproject_path.exists() {
            match project_type {
                ProjectType::Poetry(_) => perform_poetry_migration(project_dir, &mut file_tracker)?,
                ProjectType::SetupPy => perform_setup_py_migration(project_dir, &mut file_tracker)?,
                ProjectType::Pipenv => perform_pipenv_migration(project_dir, &mut file_tracker)?,
                ProjectType::Requirements => {
                    perform_requirements_migration(project_dir, &mut file_tracker)?
                }
            }
        }

        // Perform common migrations
        perform_common_migrations(
            project_dir,
            &mut file_tracker,
            import_global_pip_conf,
            additional_index_urls,
        )?;

        // Cleanup
        if hello_py_path.exists() {
            fs::remove_file(&hello_py_path)
                .map_err(|e| format!("Failed to delete hello.py: {}", e))?;
            info!("Deleted hello.py");
        }

        Ok(())
    })();

    if result.is_err() {
        info!("An error occurred during migration. Rolling back changes...");
        let migration_error = result.unwrap_err();
        file_tracker.force_rollback();
        drop(file_tracker);

        if !pyproject_path.exists() {
            return Err(format!(
                "{}\nError: Rollback failed - pyproject.toml was not restored.",
                migration_error
            ));
        }

        return Err(format!(
            "{}\nNote: File changes have been rolled back to their original state.",
            migration_error
        ));
    }

    result
}

fn perform_poetry_migration(
    project_dir: &Path,
    file_tracker: &mut FileTrackerGuard,
) -> Result<(), String> {
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
    if update_build_system(&mut doc, project_dir)? {
        info!("Migrated build system from Poetry to Hatchling");
        file_tracker.track_file(&pyproject_path)?;
        write_toml(&pyproject_path, &mut doc)?;
    }

    Ok(())
}

fn perform_setup_py_migration(
    project_dir: &Path,
    file_tracker: &mut FileTrackerGuard,
) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");

    info!("Migrating metadata from setup.py");
    if let Some(description) = SetupPyMigrationSource::extract_description(project_dir)? {
        file_tracker.track_file(&pyproject_path)?;
        pyproject::update_description(project_dir, &description)?;
    }

    info!("Migrating URL from setup.py");
    if let Some(project_url) = SetupPyMigrationSource::extract_url(project_dir)? {
        file_tracker.track_file(&pyproject_path)?;
        update_url(project_dir, &project_url)?;
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

fn perform_pipenv_migration(
    project_dir: &Path,
    file_tracker: &mut FileTrackerGuard,
) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");

    if let Ok(content) = std::fs::read_to_string(project_dir.join("Pipfile")) {
        if content.contains("[scripts]") {
            info!("Migrating Pipfile scripts");
            file_tracker.track_file(&pyproject_path)?;
        }
    }

    Ok(())
}

fn perform_requirements_migration(
    project_dir: &Path,
    file_tracker: &mut FileTrackerGuard,
) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");

    let requirements_source = requirements::RequirementsMigrationSource;
    let req_files = requirements_source.find_requirements_files(project_dir);

    for (file_path, _dep_type) in req_files {
        if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
            match file_name {
                "requirements.txt" => {
                    continue;
                }
                "requirements-dev.txt" => {
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

fn perform_common_migrations(
    project_dir: &Path,
    file_tracker: &mut FileTrackerGuard,
    import_global_pip_conf: bool,
    additional_index_urls: &[String],
) -> Result<(), String> {
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
    extra_urls.extend(additional_index_urls.iter().cloned());

    if !extra_urls.is_empty() {
        file_tracker.track_file(&pyproject_path)?;
        update_pyproject_toml(project_dir, &extra_urls)?;
    }

    info!("Migrating Tool sections");
    file_tracker.track_file(&pyproject_path)?;
    pyproject::append_tool_sections(project_dir)?;

    info!("Reordering pyproject.toml sections");
    file_tracker.track_file(&pyproject_path)?;
    crate::utils::toml::reorder_toml_sections(project_dir)?;

    Ok(())
}
