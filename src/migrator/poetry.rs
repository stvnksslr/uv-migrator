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

                // Handle dev dependencies
                if let Some(groups) = &poetry.group {
                    for (_, group) in groups {
                        for (name, value) in &group.dependencies {
                            if let Some(dep) = self.format_dependency(name, value, DependencyType::Dev) {
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
            toml::Value::String(v) => Some(v.trim_start_matches('^').to_string()),
            toml::Value::Table(t) => {
                t.get("version")
                    .and_then(|v| v.as_str())
                    .map(|v| v.trim_start_matches('^').to_string())
            }
            _ => None,
        };

        Some(Dependency {
            name: name.to_string(),
            version,
            dep_type,
        })
    }
}