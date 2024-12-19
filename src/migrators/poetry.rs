use super::{Dependency, DependencyType, MigrationSource};
use crate::migrators::detect::PoetryProjectType;
use crate::types::PyProject;
use log::{debug, info};
use std::fs;
use std::path::Path;

pub struct PoetryMigrationSource;

impl PoetryMigrationSource {
    pub fn detect_project_type(project_dir: &Path) -> Result<PoetryProjectType, String> {
        let pyproject_path = project_dir.join("pyproject.toml");
        let contents = fs::read_to_string(&pyproject_path)
            .map_err(|e| format!("Error reading file '{}': {}", pyproject_path.display(), e))?;

        let pyproject: PyProject = toml::from_str(&contents).map_err(|e| {
            format!(
                "Error parsing TOML in '{}': {}",
                pyproject_path.display(),
                e
            )
        })?;

        if let Some(tool) = &pyproject.tool {
            if let Some(poetry) = &tool.poetry {
                // Check if packages configuration exists and includes "src"
                let is_package = poetry.packages.as_ref().map_or(false, |packages| {
                    packages.iter().any(|pkg| {
                        pkg.include
                            .as_ref()
                            .map_or(false, |include| include == "src")
                    })
                });

                debug!(
                    "Poetry project type detected: {}",
                    if is_package { "package" } else { "application" }
                );
                return Ok(if is_package {
                    PoetryProjectType::Package
                } else {
                    PoetryProjectType::Application
                });
            }
        }

        debug!("No package configuration found, defaulting to application");
        Ok(PoetryProjectType::Application) // Default to application if not explicitly configured
    }

    fn format_dependency(
        &self,
        name: &str,
        value: &toml::Value,
        dep_type: DependencyType,
    ) -> Option<Dependency> {
        if name == "python" {
            debug!("Skipping python dependency");
            return None;
        }

        let version = match value {
            toml::Value::String(v) => {
                let v = v.trim();
                if v == "*" {
                    debug!("Found wildcard version for {}, setting to None", name);
                    None
                } else {
                    debug!("Found version {} for {}", v, name);
                    Some(v.to_string())
                }
            }
            toml::Value::Table(t) => {
                let version = t
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(|v| v.trim().to_string())
                    .filter(|v| v != "*");

                if let Some(ref v) = version {
                    debug!("Found version {} in table for {}", v, name);
                } else {
                    debug!("No valid version found in table for {}", name);
                }
                version
            }
            _ => {
                debug!("Unsupported value type for dependency {}", name);
                None
            }
        };

        Some(Dependency {
            name: name.to_string(),
            version,
            dep_type,
            environment_markers: None,
        })
    }
}

impl MigrationSource for PoetryMigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>, String> {
        info!("Extracting dependencies from Poetry project");
        let pyproject_path = project_dir.join("pyproject.toml");
        let contents = fs::read_to_string(&pyproject_path)
            .map_err(|e| format!("Error reading file '{}': {}", pyproject_path.display(), e))?;

        let pyproject: PyProject = toml::from_str(&contents).map_err(|e| {
            format!(
                "Error parsing TOML in '{}': {}",
                pyproject_path.display(),
                e
            )
        })?;

        let mut dependencies = Vec::new();

        if let Some(tool) = &pyproject.tool {
            if let Some(poetry) = &tool.poetry {
                // Handle main dependencies
                if let Some(deps) = &poetry.dependencies {
                    debug!("Processing main dependencies");
                    for (name, value) in deps {
                        if let Some(dep) = self.format_dependency(name, value, DependencyType::Main)
                        {
                            debug!("Added main dependency: {}", name);
                            dependencies.push(dep);
                        }
                    }
                }

                // Handle group dependencies
                if let Some(groups) = &poetry.group {
                    debug!("Processing group dependencies");
                    for (group_name, group) in groups {
                        let dep_type = match group_name.as_str() {
                            "dev" => DependencyType::Dev,
                            _ => DependencyType::Group(group_name.clone()),
                        };
                        debug!("Processing group: {}", group_name);

                        for (name, value) in &group.dependencies {
                            if let Some(dep) = self.format_dependency(name, value, dep_type.clone())
                            {
                                debug!("Added {} dependency: {}", group_name, name);
                                dependencies.push(dep);
                            }
                        }
                    }
                }
            }
        }

        info!("Extracted {} dependencies", dependencies.len());
        Ok(dependencies)
    }
}
