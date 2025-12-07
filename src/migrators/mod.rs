use crate::error::Result;
use crate::migrators::detect::detect_project_type;
use crate::models::project::{PoetryProjectType, ProjectType};
use crate::models::{Dependency, DependencyType};
use crate::utils::{file_ops::FileTrackerGuard, uv::UvCommandBuilder};
use log::info;
use semver::Version;
use std::collections::HashMap;
use std::path::Path;

pub mod common;
pub mod conda;
pub mod detect;
pub mod pipenv;
pub mod poetry;
pub mod requirements;
pub mod setup_py;

/// Trait for sources that can extract dependencies from project files
pub trait MigrationSource {
    /// Extracts dependencies from the project directory
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>>;
}

/// Trait for tools that can prepare a project and add dependencies
pub trait MigrationTool {
    /// Prepares a project for dependency management with a specific tool
    fn prepare_project(
        &self,
        project_dir: &Path,
        file_tracker: &mut FileTrackerGuard,
        project_type: &ProjectType,
    ) -> Result<()>;

    /// Adds dependencies to the project
    fn add_dependencies(&self, project_dir: &Path, dependencies: &[Dependency]) -> Result<()>;
}

/// UV migration tool implementation
pub struct UvTool;

impl MigrationTool for UvTool {
    fn prepare_project(
        &self,
        project_dir: &Path,
        file_tracker: &mut FileTrackerGuard,
        project_type: &ProjectType,
    ) -> Result<()> {
        let pyproject_path = project_dir.join("pyproject.toml");
        let backup_path = project_dir.join("old.pyproject.toml");
        let hello_py_path = project_dir.join("hello.py");

        // Backup existing pyproject.toml if it exists
        if pyproject_path.exists() {
            // Check if backup already exists to avoid overwriting previous backups
            if backup_path.exists() {
                return Err(crate::error::Error::FileOperation {
                    path: backup_path.clone(),
                    message: "Backup file 'old.pyproject.toml' already exists. Please remove or rename it before running the migration again.".to_string(),
                });
            }
            file_tracker.track_rename(&pyproject_path, &backup_path)?;
            std::fs::rename(&pyproject_path, &backup_path).map_err(|e| {
                crate::error::Error::FileOperation {
                    path: pyproject_path.clone(),
                    message: format!("Failed to rename existing pyproject.toml: {}", e),
                }
            })?;
            info!("Renamed existing pyproject.toml to old.pyproject.toml");
        }
        file_tracker.track_file(&pyproject_path)?;

        // Determine if this is a package project
        let is_package = matches!(
            project_type,
            &ProjectType::Poetry(PoetryProjectType::Package) | &ProjectType::SetupPy
        );

        // Extract Python version for Poetry and Conda projects
        let python_version = match project_type {
            ProjectType::Poetry(_) => {
                match poetry::PoetryMigrationSource::extract_python_version(project_dir)? {
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
            ProjectType::Conda => {
                match conda::CondaMigrationSource::extract_python_version_from_environment(
                    project_dir,
                )? {
                    Some(version) => {
                        info!(
                            "Found Python version constraint in Conda environment: {}",
                            version
                        );
                        Some(version)
                    }
                    None => {
                        info!("No Python version constraint found in Conda environment");
                        None
                    }
                }
            }
            _ => None,
        };

        // Use the command builder pattern
        let mut builder = UvCommandBuilder::new()?
            .arg("init")
            .working_dir(project_dir);

        if is_package {
            builder = builder.arg("--package");
        }

        if let Some(version) = python_version {
            builder = builder.arg("--python").arg(version);
        }

        // Check UV version to determine if we should use --bare flag
        let uv_version = crate::utils::uv::get_uv_version()?;
        let version_supports_bare = if let Ok(test_version) = std::env::var("UV_TEST_SUPPORT_BARE")
        {
            // Use test version during tests
            Version::parse(&test_version)
                .unwrap_or_else(|_| Version::parse(crate::utils::uv::UV_SUPPORT_BARE).unwrap())
        } else {
            // Use production version
            Version::parse(crate::utils::uv::UV_SUPPORT_BARE).unwrap()
        };

        let using_bare_flag = uv_version >= version_supports_bare;

        // Add common arguments to reduce number of files created
        builder = builder.arg("--vcs").arg("none").arg("--no-readme");

        // Add --bare flag if UV version supports it
        if using_bare_flag {
            info!(
                "Using --bare flag with UV {} to avoid hello.py creation",
                uv_version
            );
            builder = builder.arg("--bare");
        } else {
            // Only track hello.py for deletion if we're not using the --bare flag
            // as hello.py will be created in this case
            if hello_py_path.exists() {
                file_tracker.track_file(&hello_py_path)?;
            }
        }

        // Execute the command
        info!("Initializing new project with uv init");

        match builder.execute_success() {
            Ok(_) => {
                info!("Successfully initialized new project with uv init");
                Ok(())
            }
            Err(e) => Err(crate::error::Error::UvCommand(format!(
                "uv init failed: {}",
                e
            ))),
        }
    }

    fn add_dependencies(&self, project_dir: &Path, dependencies: &[Dependency]) -> Result<()> {
        // Group dependencies by type
        let mut grouped_deps: HashMap<&DependencyType, Vec<&Dependency>> = HashMap::new();
        for dep in dependencies {
            grouped_deps.entry(&dep.dep_type).or_default().push(dep);
        }

        for (dep_type, deps) in grouped_deps {
            if deps.is_empty() {
                continue;
            }

            // Start building the command
            let mut builder = UvCommandBuilder::new()?.arg("add").working_dir(project_dir);

            // Add the appropriate flags based on dependency type
            match dep_type {
                DependencyType::Dev => {
                    builder = builder.arg("--dev");
                }
                DependencyType::Group(group_name) => {
                    builder = builder.arg("--group").arg(group_name);
                }
                DependencyType::Main => {}
            }

            // Process each dependency and add it to the command
            let dep_args: Vec<String> = deps.iter().map(|dep| format_dependency(dep)).collect();

            // Add all dependency arguments
            builder = builder.args(dep_args);

            info!("Adding {:?} dependencies", dep_type);

            // Execute the command
            match builder.execute_success() {
                Ok(_) => info!("Successfully added {:?} dependencies", dep_type),
                Err(e) => {
                    return Err(crate::error::Error::UvCommand(format!(
                        "Failed to add {:?} dependencies: {}",
                        dep_type, e
                    )));
                }
            }
        }

        info!("All dependencies added successfully!");
        Ok(())
    }
}

// These functions have been moved to common.rs
use crate::utils::toml::{read_toml, update_section, write_toml};
pub use common::{
    merge_dependency_groups, perform_common_migrations, perform_conda_migration,
    perform_pipenv_migration, perform_poetry_migration, perform_requirements_migration,
    perform_setup_py_migration,
};

pub fn perform_poetry_migration_with_type(
    project_dir: &Path,
    file_tracker: &mut FileTrackerGuard,
    project_type: PoetryProjectType,
) -> Result<()> {
    // First, run the standard poetry migration
    perform_poetry_migration(project_dir, file_tracker)?;

    // Then, handle packages configuration for Poetry v2 packages
    let old_pyproject_path = project_dir.join("old.pyproject.toml");
    if old_pyproject_path.exists() && matches!(project_type, PoetryProjectType::Package) {
        let doc = read_toml(&old_pyproject_path)?;

        let packages_vec = crate::utils::pyproject::extract_poetry_packages(&doc);
        if !packages_vec.is_empty() {
            let pyproject_path = project_dir.join("pyproject.toml");
            file_tracker.track_file(&pyproject_path)?;
            let mut doc = read_toml(&pyproject_path)?;

            let mut packages_array = toml_edit::Array::new();
            for pkg in packages_vec {
                packages_array.push(toml_edit::Value::String(toml_edit::Formatted::new(pkg)));
            }

            update_section(
                &mut doc,
                &["tool", "hatch", "build", "targets", "wheel", "packages"],
                toml_edit::Item::Value(toml_edit::Value::Array(packages_array)),
            );

            write_toml(&pyproject_path, &mut doc)?;
            info!("Migrated Poetry packages configuration to Hatchling");
        }
    }

    Ok(())
}

/// Formats a dependency for use with UV command line
pub fn format_dependency(dep: &Dependency) -> String {
    // Start with base name and add extras if present
    let mut base_name = dep.name.clone();
    if let Some(extras) = &dep.extras {
        if !extras.is_empty() {
            let extras_str = extras.join(",");
            base_name = format!("{}[{}]", base_name, extras_str);
        }
    }

    // Add version formatting
    let mut dep_str = if let Some(version) = &dep.version {
        let version = version.trim();
        if version.contains(',') || version.starts_with("~=") {
            format!("{}{}", base_name, version)
        } else if let Some(stripped) = version.strip_prefix('~') {
            format!("{}~={}", base_name, stripped)
        } else if let Some(stripped) = version.strip_prefix('^') {
            format!("{}>={}", base_name, stripped)
        } else if version.starts_with(['>', '<', '=']) {
            format!("{}{}", base_name, version)
        } else {
            format!("{}=={}", base_name, version)
        }
    } else {
        base_name
    };

    // Add environment markers if present
    if let Some(markers) = &dep.environment_markers {
        dep_str.push_str(&format!("; {}", markers));
    }

    dep_str
}

/// Runs the migration process
pub fn run_migration(
    project_dir: &Path,
    import_global_pip_conf: bool,
    additional_index_urls: &[String],
    merge_groups: bool,
    restore_enabled: bool,
) -> Result<()> {
    let mut file_tracker = FileTrackerGuard::new_with_restore(restore_enabled);
    let hello_py_path = project_dir.join("hello.py");
    let pyproject_path = project_dir.join("pyproject.toml");
    let old_pyproject_path = project_dir.join("old.pyproject.toml");

    // No longer unconditionally track hello.py - this is now handled in prepare_project
    // based on whether the UV version supports the --bare flag

    let result = (|| {
        let project_type: ProjectType = detect_project_type(project_dir)?;
        info!("Detected project type: {:?}", project_type);

        // Extract dependencies based on project type
        let migration_source: Box<dyn MigrationSource> = match project_type {
            ProjectType::Poetry(_) => Box::new(poetry::PoetryMigrationSource),
            ProjectType::Pipenv => Box::new(pipenv::PipenvMigrationSource),
            ProjectType::Requirements => Box::new(requirements::RequirementsMigrationSource),
            ProjectType::SetupPy => Box::new(setup_py::SetupPyMigrationSource),
            ProjectType::Conda => Box::new(conda::CondaMigrationSource),
        };

        let mut dependencies = migration_source.extract_dependencies(project_dir)?;
        info!("Extracted {} dependencies", dependencies.len());

        if merge_groups {
            dependencies = merge_dependency_groups(dependencies);
            info!("Merged all dependency groups into dev dependencies");
        }

        // Initialize UV project
        let migration_tool = UvTool;

        // For Poetry projects, override Package type to Application if there's no actual package structure
        // BUT KEEP TRACK OF THE ORIGINAL TYPE for later package config migration
        let original_project_type = project_type.clone();
        let adjusted_project_type = match &project_type {
            ProjectType::Poetry(poetry_type) => {
                if matches!(poetry_type, PoetryProjectType::Package)
                    && !poetry::PoetryMigrationSource::verify_real_package_structure(project_dir)
                {
                    info!(
                        "Project has package configuration but lacks actual package structure - treating as application"
                    );
                    ProjectType::Poetry(PoetryProjectType::Application)
                } else {
                    project_type.clone()
                }
            }
            _ => project_type.clone(),
        };

        migration_tool.prepare_project(project_dir, &mut file_tracker, &adjusted_project_type)?;
        info!("Project initialized with UV");

        // Perform common migrations
        perform_common_migrations(
            project_dir,
            &mut file_tracker,
            import_global_pip_conf,
            additional_index_urls,
        )?;

        // Add dependencies
        migration_tool.add_dependencies(project_dir, &dependencies)?;
        info!("Dependencies added successfully");

        // Track pyproject.toml for potential updates
        file_tracker.track_file(&pyproject_path)?;

        if old_pyproject_path.exists() {
            // IMPORTANT CHANGE: Use the original_project_type for package config migration, not the adjusted one
            let migration_type = match original_project_type {
                ProjectType::Poetry(poetry_type) => poetry_type,
                _ => match &project_type {
                    ProjectType::Poetry(poetry_type) => poetry_type.clone(),
                    _ => PoetryProjectType::Application,
                },
            };

            match project_type {
                ProjectType::Poetry(_) => {
                    // Pass the original poetry type to ensure package configs are migrated properly
                    perform_poetry_migration_with_type(
                        project_dir,
                        &mut file_tracker,
                        migration_type,
                    )?
                }
                ProjectType::SetupPy => perform_setup_py_migration(project_dir, &mut file_tracker)?,
                ProjectType::Pipenv => perform_pipenv_migration(project_dir, &mut file_tracker)?,
                ProjectType::Requirements => {
                    perform_requirements_migration(project_dir, &mut file_tracker)?
                }
                ProjectType::Conda => perform_conda_migration(project_dir, &mut file_tracker)?,
            }
        } else if matches!(project_type, ProjectType::Conda) {
            // For Conda projects without existing pyproject.toml
            perform_conda_migration(project_dir, &mut file_tracker)?;
        }

        // Cleanup
        if hello_py_path.exists() {
            std::fs::remove_file(&hello_py_path).map_err(|e| {
                crate::error::Error::FileOperation {
                    path: hello_py_path.clone(),
                    message: format!("Failed to delete hello.py: {}", e),
                }
            })?;
            info!("Deleted hello.py");
        }

        Ok(())
    })();

    if let Err(error) = &result {
        info!("An error occurred during migration. Rolling back changes...");
        file_tracker.force_rollback();
        drop(file_tracker);

        if !pyproject_path.exists() {
            return Err(crate::error::Error::General(format!(
                "{}\nError: Rollback failed - pyproject.toml was not restored.",
                error
            )));
        }

        return Err(crate::error::Error::General(format!(
            "{}\nNote: File changes have been rolled back to their original state.",
            error
        )));
    }

    result
}
