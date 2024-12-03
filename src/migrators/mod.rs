use crate::utils::{
    create_virtual_environment, parse_pip_conf, update_pyproject_toml, FileTrackerGuard,
};
use log::info;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

mod dependency;
mod detect;
pub mod poetry;
mod pyproject;
pub mod requirements;

pub use dependency::{Dependency, DependencyType};
pub use detect::{detect_project_type, ProjectType};

pub trait MigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>, String>;
}

pub trait MigrationTool {
    fn prepare_project(
        &self,
        project_dir: &Path,
        file_tracker: &mut FileTrackerGuard,
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
    ) -> Result<(), String> {
        let pyproject_path = project_dir.join("pyproject.toml");
        let backup_path = project_dir.join("old.pyproject.toml");

        if pyproject_path.exists() {
            file_tracker.track_rename(&pyproject_path, &backup_path)?;
            fs::rename(&pyproject_path, &backup_path)
                .map_err(|e| format!("Failed to rename existing pyproject.toml: {}", e))?;
            info!("Renamed existing pyproject.toml to old.pyproject.toml");
        }

        file_tracker.track_file(&pyproject_path)?;
        info!("Initializing new project with uv init");

        let uv_path =
            which::which("uv").map_err(|e| format!("Failed to find uv command: {}", e))?;

        let output = std::process::Command::new(&uv_path)
            .arg("init")
            .arg("--no-pin-python")
            .current_dir(project_dir)
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

pub fn run_migration(
    project_dir: &Path,
    import_global_pip_conf: bool,
    additional_index_urls: &[String],
) -> Result<(), String> {
    let mut file_tracker = FileTrackerGuard::new();
    let hello_py_path = project_dir.join("hello.py");
    let pyproject_path = project_dir.join("pyproject.toml");

    if hello_py_path.exists() {
        file_tracker.track_file(&hello_py_path)?;
    }

    let result = (|| {
        create_virtual_environment()?;
        let project_type = detect_project_type(project_dir)?;
        info!("Detected project type: {:?}", project_type);

        let migration_source: Box<dyn MigrationSource> = match project_type {
            ProjectType::Poetry => Box::new(poetry::PoetryMigrationSource),
            ProjectType::Requirements => Box::new(requirements::RequirementsMigrationSource),
        };

        let dependencies = migration_source.extract_dependencies(project_dir)?;
        info!("Extracted {} dependencies", dependencies.len());

        let migration_tool = UvTool;
        migration_tool.prepare_project(project_dir, &mut file_tracker)?;

        let mut extra_urls = Vec::new();
        if import_global_pip_conf {
            extra_urls.extend(parse_pip_conf()?);
        }
        extra_urls.extend(additional_index_urls.iter().cloned());

        if !extra_urls.is_empty() {
            file_tracker.track_file(&pyproject_path)?;
            update_pyproject_toml(project_dir, &extra_urls)?;
        }

        migration_tool.add_dependencies(project_dir, &dependencies)?;

        file_tracker.track_file(&pyproject_path)?;
        pyproject::append_tool_sections(project_dir)?;

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
