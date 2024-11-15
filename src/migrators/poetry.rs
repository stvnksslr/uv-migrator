// In migrators/poetry.rs

use super::{MigrationSource, Dependency, DependencyType};
use crate::types::PyProject;
use std::path::Path;
use std::fs;

pub struct PoetryMigrationSource;

impl MigrationSource for PoetryMigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>, String> {
        let pyproject_path = project_dir.join("pyproject.toml");
        let contents = fs::read_to_string(&pyproject_path)
            .map_err(|e| format!("Error reading file '{}': {}", pyproject_path.display(), e))?;

        let pyproject: PyProject = toml::from_str(&contents)
            .map_err(|e| format!("Error parsing TOML in '{}': {}", pyproject_path.display(), e))?;

        let mut dependencies = Vec::new();

        if let Some(tool) = &pyproject.tool {
            if let Some(poetry) = &tool.poetry {
                // Handle main dependencies
                if let Some(deps) = &poetry.dependencies {
                    for (name, value) in deps {
                        if let Some(dep) = self.format_dependency(name, value, DependencyType::Main) {
                            dependencies.push(dep);
                        }
                    }
                }

                // Handle group dependencies
                if let Some(groups) = &poetry.group {
                    for (group_name, group) in groups {
                        // Determine dependency type based on group name
                        let dep_type = match group_name.as_str() {
                            "dev" | "test" => DependencyType::Dev,
                            _ => DependencyType::Group(group_name.clone())
                        };

                        for (name, value) in &group.dependencies {
                            if let Some(dep) = self.format_dependency(name, value, dep_type.clone()) {
                                dependencies.push(dep);
                            }
                        }
                    }
                }
            }
        }

        Ok(dependencies)
    }
}

impl PoetryMigrationSource {
    fn format_dependency(&self, name: &str, value: &toml::Value, dep_type: DependencyType) -> Option<Dependency> {
        if name == "python" {
            return None;
        }

        let version = match value {
            toml::Value::String(v) => {
                let v = v.trim();
                if v == "*" {
                    None
                } else {
                    Some(v.to_string())
                }
            },
            toml::Value::Table(t) => {
                t.get("version")
                    .and_then(|v| v.as_str())
                    .map(|v| v.trim().to_string())
                    .filter(|v| v != "*")
            }
            _ => None,
        };

        Some(Dependency {
            name: name.to_string(),
            version,
            dep_type,
            environment_markers: None,
        })
    }
}