// src/migrators/pipenv.rs
use super::{Dependency, DependencyType, MigrationSource};
use log::{debug, info};
use std::fs;
use std::path::Path;
use toml_edit::DocumentMut;

pub struct PipenvMigrationSource;

impl PipenvMigrationSource {
    pub fn detect_project_type(project_dir: &Path) -> bool {
        project_dir.join("Pipfile").exists()
    }

    fn extract_version_and_markers(
        &self,
        value: &toml_edit::Item,
    ) -> (Option<String>, Option<String>) {
        match value {
            toml_edit::Item::Value(toml_edit::Value::String(s)) => {
                let v = s.value().trim();
                if v == "*" {
                    (None, None)
                } else {
                    (Some(v.to_string()), None)
                }
            }
            toml_edit::Item::Table(table) => {
                let mut version = None;
                let mut markers = Vec::new();

                // Handle version specification
                if let Some(v) = table.get("version").and_then(|v| v.as_str()) {
                    if v != "*" {
                        version = Some(v.to_string());
                    }
                }

                // Handle git references
                if let Some(git_url) = table.get("git").and_then(|g| g.as_str()) {
                    if let Some(ref_value) = table.get("ref").and_then(|r| r.as_str()) {
                        version = Some(format!("git+{}@{}", git_url, ref_value));
                    } else {
                        version = Some(format!("git+{}", git_url));
                    }
                }

                // Handle path-based dependencies
                if let Some(path_str) = table.get("path").and_then(|p| p.as_str()) {
                    version = Some(format!("path/{}", path_str));
                }

                // Handle platform markers
                if let Some(platform) = table.get("sys_platform").and_then(|p| p.as_str()) {
                    markers.push(format!("sys_platform {}", platform));
                }

                // Handle Python version markers
                if let Some(python_marker) = table.get("markers").and_then(|m| m.as_str()) {
                    markers.push(python_marker.to_string());
                }

                // Handle extras
                if let Some(extras) = table.get("extras").and_then(|e| e.as_array()) {
                    let extras_str: Vec<String> = extras
                        .iter()
                        .filter_map(|e| e.as_str().map(|s| s.to_string()))
                        .collect();
                    if !extras_str.is_empty() {
                        if let Some(ref mut v) = version {
                            v.push_str(&format!("[{}]", extras_str.join(",")));
                        }
                    }
                }

                let combined_markers = if !markers.is_empty() {
                    Some(markers.join(" and "))
                } else {
                    None
                };

                (version, combined_markers)
            }
            _ => (None, None),
        }
    }

    fn parse_packages(
        &self,
        packages_table: &toml_edit::Table,
        dep_type: DependencyType,
    ) -> Vec<Dependency> {
        let mut dependencies = Vec::new();

        for (name, value) in packages_table.iter() {
            // Skip python version requirements and script entries
            if name == "python_version" || name == "python_full_version" {
                continue;
            }

            let (version, environment_markers) = self.extract_version_and_markers(value);

            dependencies.push(Dependency {
                name: name.to_string(),
                version,
                dep_type: dep_type.clone(),
                environment_markers,
            });
        }

        dependencies
    }
}

impl MigrationSource for PipenvMigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>, String> {
        info!("Extracting dependencies from Pipenv project");
        let pipfile_path = project_dir.join("Pipfile");

        if !pipfile_path.exists() {
            return Err(format!("Error reading file '{}'", pipfile_path.display()));
        }

        let content = fs::read_to_string(&pipfile_path)
            .map_err(|e| format!("Error reading file '{}': {}", pipfile_path.display(), e))?;

        let doc = content
            .parse::<DocumentMut>()
            .map_err(|e| format!("Error parsing TOML in '{}': {}", pipfile_path.display(), e))?;

        let mut dependencies = Vec::new();

        // Extract main packages
        if let Some(packages) = doc.get("packages").and_then(|p| p.as_table()) {
            debug!("Processing main dependencies");
            dependencies.extend(self.parse_packages(packages, DependencyType::Main));
        }

        // Extract dev packages
        if let Some(dev_packages) = doc.get("dev-packages").and_then(|p| p.as_table()) {
            debug!("Processing dev dependencies");
            dependencies.extend(self.parse_packages(dev_packages, DependencyType::Dev));
        }

        info!("Extracted {} dependencies", dependencies.len());
        Ok(dependencies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_project(content: &str) -> (TempDir, std::path::PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();
        let pipfile_path = project_dir.join("Pipfile");
        fs::write(&pipfile_path, content).unwrap();
        (temp_dir, project_dir)
    }

    #[test]
    fn test_complex_dependencies() {
        let content = r#"
[[source]]
url = "https://pypi.org/simple"
verify_ssl = true
name = "pypi"

[dev-packages]
pipenv = {path = ".", editable = true, extras = ["tests", "dev"]}
sphinx-click = "==4.*"
stdeb = {version="*", sys_platform = "== 'linux'"}
zipp = {version = "==3.6.0", markers = "python_version < '3.10'"}
pypiserver = {ref = "pipenv-313", git = "https://github.com/matteius/pypiserver.git"}
myst-parser = {extras = ["linkify"], version = "*"}

[packages]
pytz = "*"

[scripts]
tests = "bash ./run-tests.sh"
"#;

        let (_temp_dir, project_dir) = create_test_project(content);
        let source = PipenvMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        let pipenv_dep = dependencies.iter().find(|d| d.name == "pipenv").unwrap();
        assert!(pipenv_dep.version.as_ref().unwrap().contains("path/."));
        assert!(matches!(pipenv_dep.dep_type, DependencyType::Dev));

        let sphinx_click_dep = dependencies
            .iter()
            .find(|d| d.name == "sphinx-click")
            .unwrap();
        assert_eq!(sphinx_click_dep.version, Some("==4.*".to_string()));

        let stdeb_dep = dependencies.iter().find(|d| d.name == "stdeb").unwrap();
        assert!(stdeb_dep
            .environment_markers
            .as_ref()
            .unwrap()
            .contains("linux"));

        let zipp_dep = dependencies.iter().find(|d| d.name == "zipp").unwrap();
        assert_eq!(zipp_dep.version, Some("==3.6.0".to_string()));
        assert!(zipp_dep
            .environment_markers
            .as_ref()
            .unwrap()
            .contains("python_version < '3.10'"));

        let pypiserver_dep = dependencies
            .iter()
            .find(|d| d.name == "pypiserver")
            .unwrap();
        assert!(pypiserver_dep
            .version
            .as_ref()
            .unwrap()
            .contains("git+https"));
        assert!(pypiserver_dep
            .version
            .as_ref()
            .unwrap()
            .contains("@pipenv-313"));

        let myst_parser_dep = dependencies
            .iter()
            .find(|d| d.name == "myst-parser")
            .unwrap();
        assert!(myst_parser_dep.version.is_none());
    }

    #[test]
    fn test_platform_specific_dependencies() {
        let content = r#"
[dev-packages]
atomicwrites = {version = "*", sys_platform = "== 'win32'"}
gunicorn = {version = "*", sys_platform = "== 'linux'"}
"#;

        let (_temp_dir, project_dir) = create_test_project(content);
        let source = PipenvMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        let atomicwrites_dep = dependencies
            .iter()
            .find(|d| d.name == "atomicwrites")
            .unwrap();
        assert!(atomicwrites_dep
            .environment_markers
            .as_ref()
            .unwrap()
            .contains("win32"));

        let gunicorn_dep = dependencies.iter().find(|d| d.name == "gunicorn").unwrap();
        assert!(gunicorn_dep
            .environment_markers
            .as_ref()
            .unwrap()
            .contains("linux"));
    }

    #[test]
    fn test_ignore_scripts_section() {
        let content = r#"
[packages]
flask = "*"

[scripts]
tests = "pytest"
serve = "flask run"
"#;

        let (_temp_dir, project_dir) = create_test_project(content);
        let source = PipenvMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0].name, "flask");
    }

    #[test]
    fn test_git_dependencies() {
        let content = r#"
[dev-packages]
pypiserver = {ref = "pipenv-313", git = "https://github.com/matteius/pypiserver.git"}
"#;

        let (_temp_dir, project_dir) = create_test_project(content);
        let source = PipenvMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        let pypiserver_dep = dependencies
            .iter()
            .find(|d| d.name == "pypiserver")
            .unwrap();
        assert!(pypiserver_dep
            .version
            .as_ref()
            .unwrap()
            .contains("git+https"));
        assert!(pypiserver_dep
            .version
            .as_ref()
            .unwrap()
            .contains("@pipenv-313"));
    }
}
