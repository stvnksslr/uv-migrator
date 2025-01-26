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

        // First, check the project section (Poetry 2.0 style)
        if let Some(project) = doc.get("project") {
            // If project section has dependencies, it's likely a Poetry 2.0 package
            if project.get("dependencies").is_some() {
                return Ok(PoetryProjectType::Package);
            }
        }

        // Traditional Poetry style detection
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

        // First, check project section (Poetry 2.0 style)
        if let Some(project) = doc.get("project") {
            if let Some(python_dep) = project.get("requires-python").and_then(|p| p.as_str()) {
                // Extract the minimum version from various formats
                let version = if let Some(stripped) = python_dep.strip_prefix(">=") {
                    stripped.split(',').next().unwrap_or(stripped)
                } else if let Some(stripped) = python_dep.strip_prefix("^") {
                    stripped
                } else if let Some(stripped) = python_dep.strip_prefix("~=") {
                    stripped
                } else {
                    python_dep.split(&[',', ' ']).next().unwrap_or(python_dep)
                };

                // Extract major.minor
                let parts: Vec<&str> = version.split('.').collect();
                let normalized_version = match parts.len() {
                    0 => return Ok(None),
                    1 => format!("{}.0", parts[0]),
                    _ => parts.into_iter().take(2).collect::<Vec<_>>().join("."),
                };

                return Ok(Some(normalized_version));
            }
        }

        // If not found in project section, fall back to tool.poetry section
        if let Some(tool) = doc.get("tool") {
            if let Some(poetry) = tool.get("poetry") {
                if let Some(deps) = poetry.get("dependencies") {
                    if let Some(python_dep) = deps.get("python") {
                        let version_str = match python_dep {
                            Item::Value(Value::String(s)) => s.value().trim().to_string(),
                            _ => return Ok(None),
                        };

                        // Extract the minimum version from various formats
                        let version = if let Some(stripped) = version_str.strip_prefix(">=") {
                            stripped.split(',').next().unwrap_or(stripped)
                        } else if let Some(stripped) = version_str.strip_prefix("^") {
                            stripped
                        } else if let Some(stripped) = version_str.strip_prefix("~=") {
                            stripped
                        } else {
                            version_str
                                .split(&[',', ' '])
                                .next()
                                .unwrap_or(&version_str)
                        };

                        // Extract major.minor
                        let parts: Vec<&str> = version.split('.').collect();
                        let normalized_version = match parts.len() {
                            0 => return Ok(None),
                            1 => format!("{}.0", parts[0]),
                            _ => parts.into_iter().take(2).collect::<Vec<_>>().join("."),
                        };

                        return Ok(Some(normalized_version));
                    }
                }
            }
        }

        Ok(None)
    }

    fn parse_poetry_v2_dep(&self, dep_str: &str) -> (String, Option<String>) {
        // Split the dependency string to extract name and version
        let parts: Vec<&str> = dep_str.split_whitespace().collect();

        match parts.len() {
            1 => (parts[0].to_string(), None), // No version specified
            2 => {
                // Version is specified, potentially with comparison operators
                let name = parts[0].to_string();
                let version = parts[1].trim_matches(&['(', ')'][..]).to_string();
                (name, Some(version))
            }
            _ => {
                // Fallback for unexpected formats
                debug!("Unexpected dependency format: {}", dep_str);
                (dep_str.to_string(), None)
            }
        }
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

        // First, check the project section (Poetry 2.0 style)
        if let Some(project) = doc.get("project") {
            // Extract dependencies from project section
            if let Some(proj_deps) = project.get("dependencies").and_then(|d| d.as_array()) {
                debug!("Processing main dependencies from project section");
                for dep_value in proj_deps.iter() {
                    if let Some(dep_str) = dep_value.as_str() {
                        // Split the dependency string into name and version
                        let (name, version) = self.parse_poetry_v2_dep(dep_str);

                        let dep = Dependency {
                            name,
                            version,
                            dep_type: DependencyType::Main,
                            environment_markers: None,
                        };

                        dependencies.push(dep);
                    }
                }
            }
        }

        // Then, check the tool.poetry section (traditional Poetry style)
        if let Some(tool) = doc.get("tool") {
            if let Some(poetry) = tool.get("poetry") {
                // Handle main dependencies
                if let Some(deps) = poetry.get("dependencies").and_then(|d| d.as_table()) {
                    debug!("Processing main dependencies from tool.poetry section");
                    for (name, value) in deps.iter() {
                        if let Some(dep) = self.format_dependency(name, value, DependencyType::Main)
                        {
                            debug!("Added main dependency: {}", name);
                            // Avoid duplicates
                            if !dependencies
                                .iter()
                                .any(|existing| existing.name == dep.name)
                            {
                                dependencies.push(dep);
                            }
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
