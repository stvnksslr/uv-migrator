mod detect;
mod poetry;
mod requirements;
mod pyproject;
mod dependency;

use crate::utils::create_virtual_environment;
pub use detect::detect_project_type;
pub use dependency::{Dependency, DependencyType};
use std::path::Path;
use std::fs;
use log::info;

pub trait MigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>, String>;
}

pub trait MigrationTool {
    fn prepare_project(&self, project_dir: &Path) -> Result<(), String>;
    fn add_dependencies(&self, project_dir: &Path, dependencies: &[Dependency]) -> Result<(), String>;
}

pub struct UvTool;

impl MigrationTool for UvTool {
    fn prepare_project(&self, project_dir: &Path) -> Result<(), String> {
        let pyproject_path = project_dir.join("pyproject.toml");

        if pyproject_path.exists() {
            let backup_path = project_dir.join("old.pyproject.toml");
            fs::rename(&pyproject_path, &backup_path)
                .map_err(|e| format!("Failed to rename existing pyproject.toml: {}", e))?;
            info!("Renamed existing pyproject.toml to old.pyproject.toml");
        }

        info!("Initializing new project with uv init");
        let uv_path = which::which("uv").map_err(|e| format!("Failed to find uv command: {}", e))?;
        let output = std::process::Command::new(&uv_path)
            .arg("init")
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

    fn add_dependencies(&self, project_dir: &Path, dependencies: &[Dependency]) -> Result<(), String> {
        let uv_path = which::which("uv").map_err(|e| format!("Failed to find uv command: {}", e))?;

        for dep_type in &[DependencyType::Main, DependencyType::Dev] {
            let deps: Vec<_> = dependencies.iter()
                .filter(|d| d.dep_type == *dep_type)
                .collect();

            if deps.is_empty() {
                continue;
            }

            let mut command = std::process::Command::new(&uv_path);
            command.arg("add");
            if *dep_type == DependencyType::Dev {
                command.arg("--dev");
            }
            command.current_dir(project_dir);

            for dep in deps {
                let mut dep_str = if let Some(version) = &dep.version {
                    format!("{}=={}", dep.name, version)
                } else {
                    dep.name.clone()
                };

                if let Some(markers) = &dep.environment_markers {
                    dep_str.push_str(&format!("; {}", markers));
                }

                command.arg(dep_str);
            }

            let output = command
                .output()
                .map_err(|e| format!("Failed to execute uv command: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Failed to add {:?} dependencies: {}", dep_type, stderr));
            }
        }

        info!("All dependencies added successfully!");
        Ok(())
    }
}

pub fn run_migration(project_dir: &Path) -> Result<(), String> {
    create_virtual_environment()?;

    let project_type = detect_project_type(project_dir)?;
    info!("Detected project type: {:?}", project_type);

    let migration_source: Box<dyn MigrationSource> = match project_type {
        detect::ProjectType::Poetry => Box::new(poetry::PoetryMigrationSource),
        detect::ProjectType::Requirements => Box::new(requirements::RequirementsMigrationSource),
    };

    let dependencies = migration_source.extract_dependencies(project_dir)?;
    info!("Extracted {} dependencies", dependencies.len());

    let migration_tool = UvTool;
    migration_tool.prepare_project(project_dir)?;
    migration_tool.add_dependencies(project_dir, &dependencies)?;

    if let detect::ProjectType::Poetry = project_type {
        pyproject::append_tool_sections(project_dir)?;
    }

    let hello_py_path = project_dir.join("hello.py");
    if hello_py_path.exists() {
        fs::remove_file(&hello_py_path)
            .map_err(|e| format!("Failed to delete hello.py: {}", e))?;
        info!("Deleted hello.py");
    }

    Ok(())
}