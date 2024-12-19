// src/migrators/setup_py.rs

use super::requirements::RequirementsMigrationSource;
use super::{Dependency, DependencyType, MigrationSource};
use log::{debug, info};
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct SetupPyMigrationSource;

impl MigrationSource for SetupPyMigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>, String> {
        info!("Extracting dependencies from setup.py");

        // First check if we have requirements files
        let requirements_source = RequirementsMigrationSource;
        if requirements_source.has_requirements_files(project_dir) {
            info!("Found requirements files, using requirements parser");
            return requirements_source.extract_dependencies(project_dir);
        }

        // If no requirements files found, fall back to setup.py parsing
        info!("No requirements files found, falling back to setup.py parsing");
        self.extract_deps_from_setup_py(project_dir)
    }
}

impl SetupPyMigrationSource {
    fn extract_deps_from_setup_py(&self, project_dir: &Path) -> Result<Vec<Dependency>, String> {
        let venv_path = project_dir.join(".setuppy-temp-venv");
        self.create_temp_venv(&venv_path)?;

        let result = self.extract_deps_using_pip(project_dir);

        if venv_path.exists() {
            std::fs::remove_dir_all(&venv_path)
                .map_err(|e| format!("Failed to clean up temporary venv: {}", e))?;
        }

        result
    }

    fn create_temp_venv(&self, venv_path: &Path) -> Result<(), String> {
        debug!("Creating temporary venv at {}", venv_path.display());

        let uv_path =
            which::which("uv").map_err(|e| format!("Failed to find uv executable: {}", e))?;

        let output = Command::new(&uv_path)
            .args(["venv", &venv_path.to_string_lossy()])
            .output()
            .map_err(|e| format!("Failed to create virtual environment: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to create virtual environment: {}", stderr));
        }

        Ok(())
    }

    fn extract_deps_using_pip(&self, project_dir: &Path) -> Result<Vec<Dependency>, String> {
        debug!("Extracting dependencies using pip");

        let uv_path =
            which::which("uv").map_err(|e| format!("Failed to find uv executable: {}", e))?;

        let output = Command::new(&uv_path)
            .args(["pip", "install", "--no-deps", "--dry-run", "-e", "."])
            .current_dir(project_dir)
            .output()
            .map_err(|e| format!("Failed to extract dependencies: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to extract dependencies: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut dependencies = Vec::new();

        for line in stdout.lines() {
            if let Some(dep) = self.parse_dependency_line(line) {
                dependencies.push(dep);
            }
        }

        // Also try to extract test dependencies
        let test_deps = self.extract_test_dependencies(project_dir)?;
        dependencies.extend(test_deps);

        Ok(dependencies)
    }

    fn parse_dependency_line(&self, line: &str) -> Option<Dependency> {
        let line = line.trim();
        if !line.contains("Collecting") {
            return None;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }

        let dep_str = parts[1];
        if dep_str == "setuptools" {
            return None;
        }

        let (name, version) = if dep_str.contains("==") {
            let parts: Vec<&str> = dep_str.split("==").collect();
            (parts[0].to_string(), Some(parts[1].to_string()))
        } else {
            (dep_str.to_string(), None)
        };

        Some(Dependency {
            name,
            version,
            dep_type: DependencyType::Main,
            environment_markers: None,
        })
    }

    fn extract_test_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>, String> {
        let setup_py_content = fs::read_to_string(project_dir.join("setup.py"))
            .map_err(|e| format!("Failed to read setup.py: {}", e))?;

        let mut test_deps = Vec::new();

        for line in setup_py_content.lines() {
            let line = line.trim();
            if line.contains("tests_require") || line.contains("setup_requires") {
                if let Some(dep) = self.parse_setup_py_dep_line(line) {
                    test_deps.push(Dependency {
                        name: dep,
                        version: None,
                        dep_type: DependencyType::Dev,
                        environment_markers: None,
                    });
                }
            }
        }

        Ok(test_deps)
    }

    fn parse_setup_py_dep_line(&self, line: &str) -> Option<String> {
        if line.contains('\'') || line.contains('"') {
            let start = line.find(['\'', '"'])?;
            let end = line[start + 1..].find(['\'', '"'])?;
            Some(line[start + 1..start + 1 + end].to_string())
        } else {
            None
        }
    }
}
