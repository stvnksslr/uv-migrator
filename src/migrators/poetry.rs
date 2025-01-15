use super::{Dependency, DependencyType, MigrationSource};
use crate::migrators::detect::PoetryProjectType;
use crate::utils::toml::read_toml;
use log::{debug, info};
use std::fs;
use std::path::Path;
use toml_edit::{DocumentMut, Item, Value};

pub struct PoetryMigrationSource;

impl PoetryMigrationSource {
    pub fn detect_project_type(project_dir: &Path) -> Result<PoetryProjectType, String> {
        let pyproject_path = project_dir.join("pyproject.toml");
        let doc = read_toml(&pyproject_path)?;

        if let Some(tool) = doc.get("tool") {
            if let Some(poetry) = tool.get("poetry") {
                let is_package = poetry
                    .get("packages")
                    .and_then(|packages| packages.as_array())
                    .is_some_and(|packages| {
                        packages.iter().any(|pkg| {
                            pkg.as_inline_table()
                                .and_then(|t| t.get("include"))
                                .and_then(|i| i.as_str())
                                == Some("src")
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
        Ok(PoetryProjectType::Application)
    }

    pub fn extract_python_version(project_dir: &Path) -> Result<Option<String>, String> {
        let old_pyproject_path = project_dir.join("old.pyproject.toml");
        if !old_pyproject_path.exists() {
            return Ok(None);
        }

        let doc = read_toml(&old_pyproject_path)?;

        if let Some(tool) = doc.get("tool") {
            if let Some(poetry) = tool.get("poetry") {
                if let Some(deps) = poetry.get("dependencies") {
                    if let Some(python_dep) = deps.get("python") {
                        let version_str = match python_dep {
                            Item::Value(Value::String(s)) => s.value().trim().to_string(),
                            _ => return Ok(None),
                        };

                        // Clean up the version string
                        let version = version_str.trim_matches('"');

                        // Convert poetry's ^3.9 format to 3.9
                        if let Some(stripped) = version.strip_prefix('^') {
                            return Ok(Some(stripped.to_string()));
                        }

                        // Handle >=3.9 format
                        if let Some(stripped) = version.strip_prefix(">=") {
                            return Ok(Some(stripped.to_string()));
                        }

                        // Handle ~=3.9 format
                        if let Some(stripped) = version.strip_prefix("~=") {
                            return Ok(Some(stripped.to_string()));
                        }

                        // Handle version ranges by extracting the minimum version
                        if version.contains(',') {
                            let min_version = version
                                .split(',')
                                .next()
                                .and_then(|v| v.trim().strip_prefix(">="))
                                .map(|v| v.trim().to_string());

                            if let Some(version) = min_version {
                                return Ok(Some(version));
                            }
                        }

                        // If no special prefix, return as-is
                        return Ok(Some(version.to_string()));
                    }
                }
            }
        }

        Ok(None)
    }

    fn format_dependency(
        &self,
        name: &str,
        value: &Item,
        dep_type: DependencyType,
    ) -> Option<Dependency> {
        if name == "python" {
            debug!("Skipping python dependency");
            return None;
        }

        let version = match value {
            Item::Value(Value::String(v)) => {
                let v = v.value().trim();
                if v == "*" {
                    None
                } else {
                    Some(v.to_string())
                }
            }
            Item::Value(Value::InlineTable(t)) => t.get("version").and_then(|v| match v {
                Value::String(s) => {
                    let version = s.value().trim();
                    if version == "*" {
                        None
                    } else {
                        Some(version.to_string())
                    }
                }
                _ => None,
            }),
            Item::Table(t) => t.get("version").and_then(|v| match v {
                Item::Value(Value::String(s)) => {
                    let version = s.value().trim();
                    if version == "*" {
                        None
                    } else {
                        Some(version.to_string())
                    }
                }
                _ => None,
            }),
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

impl MigrationSource for PoetryMigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>, String> {
        info!("Extracting dependencies from Poetry project");
        let pyproject_path = project_dir.join("pyproject.toml");

        if !pyproject_path.exists() {
            return Err(format!("Error reading file '{}'", pyproject_path.display()));
        }

        let content = fs::read_to_string(&pyproject_path)
            .map_err(|e| format!("Error reading file '{}': {}", pyproject_path.display(), e))?;

        let doc = content.parse::<DocumentMut>().map_err(|e| {
            format!(
                "Error parsing TOML in '{}': {}",
                pyproject_path.display(),
                e
            )
        })?;

        let mut dependencies = Vec::new();

        if let Some(tool) = doc.get("tool") {
            if let Some(poetry) = tool.get("poetry") {
                // Handle main dependencies
                if let Some(deps) = poetry.get("dependencies").and_then(|d| d.as_table()) {
                    debug!("Processing main dependencies");
                    for (name, value) in deps.iter() {
                        if let Some(dep) = self.format_dependency(name, value, DependencyType::Main)
                        {
                            debug!("Added main dependency: {}", name);
                            dependencies.push(dep);
                        }
                    }
                }

                // Handle group dependencies
                if let Some(groups) = poetry.get("group").and_then(|g| g.as_table()) {
                    debug!("Processing group dependencies");
                    for (group_name, group) in groups.iter() {
                        let dep_type = match group_name {
                            "dev" => DependencyType::Dev,
                            _ => DependencyType::Group(group_name.to_string()),
                        };
                        debug!("Processing group: {}", group_name);

                        if let Some(deps) = group
                            .as_table()
                            .and_then(|g| g.get("dependencies"))
                            .and_then(|d| d.as_table())
                        {
                            for (name, value) in deps.iter() {
                                if let Some(dep) =
                                    self.format_dependency(name, value, dep_type.clone())
                                {
                                    debug!("Added {} dependency: {}", group_name, name);
                                    dependencies.push(dep);
                                }
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
