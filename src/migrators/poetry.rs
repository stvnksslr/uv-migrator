use super::{Dependency, DependencyType, MigrationSource};
use crate::migrators::detect::PoetryProjectType;
use crate::utils::toml::read_toml;
use log::{debug, info};
use std::path::Path;
use toml_edit::{Item, Value};

pub struct PoetryMigrationSource;

impl PoetryMigrationSource {
    pub fn detect_project_type(project_dir: &Path) -> Result<PoetryProjectType, String> {
        let pyproject_path = project_dir.join("pyproject.toml");
        let doc = read_toml(&pyproject_path)?;

        if let Some(tool) = doc.get("tool") {
            if let Some(poetry) = tool.get("poetry") {
                // Check if packages configuration exists and includes "src"
                let is_package = poetry
                    .get("packages")
                    .and_then(|packages| packages.as_array())
                    .map_or(false, |packages| {
                        packages.iter().any(|pkg| {
                            pkg.as_inline_table()
                                .and_then(|t| t.get("include"))
                                .and_then(|i| i.as_str())
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
        Ok(PoetryProjectType::Application)
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
                    debug!("Found wildcard version for {}, setting to None", name);
                    None
                } else {
                    debug!("Found version {} for {}", v, name);
                    Some(v.to_string())
                }
            }
            Item::Table(t) => {
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
        let doc = read_toml(&pyproject_path)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_project(content: &str) -> (TempDir, std::path::PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();
        let pyproject_path = project_dir.join("pyproject.toml");
        fs::write(&pyproject_path, content).unwrap();
        (temp_dir, project_dir)
    }

    #[test]
    fn test_extract_main_dependencies() {
        let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.11"
fastapi = "^0.111.0"
aiofiles = "24.1.0"
jinja2 = { version = "^3.1.4" }
uvicorn = { extras = ["standard"], version = "^0.30.1" }
"#;
        let (_temp_dir, project_dir) = create_test_project(content);

        let source = PoetryMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        assert_eq!(dependencies.len(), 4); // Should not include python

        let fastapi_dep = dependencies.iter().find(|d| d.name == "fastapi").unwrap();
        assert_eq!(fastapi_dep.version, Some("^0.111.0".to_string()));
        assert_eq!(fastapi_dep.dep_type, DependencyType::Main);

        let aiofiles_dep = dependencies.iter().find(|d| d.name == "aiofiles").unwrap();
        assert_eq!(aiofiles_dep.version, Some("24.1.0".to_string()));
        assert_eq!(aiofiles_dep.dep_type, DependencyType::Main);
    }

    #[test]
    fn test_extract_dev_dependencies() {
        let content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.group.dev.dependencies]
pytest = "^8.2.2"
pytest-cov = "^5.0.0"
pytest-sugar = "^1.0.0"
"#;
        let (_temp_dir, project_dir) = create_test_project(content);

        let source = PoetryMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        assert_eq!(dependencies.len(), 3);
        for dep in dependencies {
            assert!(matches!(dep.dep_type, DependencyType::Dev));
        }
    }

    #[test]
    fn test_detect_project_type() {
        let package_content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.packages]
include = "src"
"#;
        let (_temp_dir, project_dir) = create_test_project(package_content);
        let result = PoetryMigrationSource::detect_project_type(&project_dir).unwrap();
        assert!(matches!(result, PoetryProjectType::Package));

        let app_content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"
"#;
        let (_temp_dir, project_dir) = create_test_project(app_content);
        let result = PoetryMigrationSource::detect_project_type(&project_dir).unwrap();
        assert!(matches!(result, PoetryProjectType::Application));
    }
}
