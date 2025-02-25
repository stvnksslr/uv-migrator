use crate::error::Result;
use crate::migrators::MigrationSource;
use crate::models::dependency::{Dependency, DependencyType};
use log::{debug, info};
use serde_json::Value;
use std::{fs, path::Path};

pub struct PipenvMigrationSource;

impl PipenvMigrationSource {
    pub fn detect_project_type(project_dir: &Path) -> bool {
        project_dir.join("Pipfile.lock").exists()
    }

    fn parse_dependency(
        &self,
        name: &str,
        value: &Value,
        dep_type: DependencyType,
    ) -> Result<Option<Dependency>> {
        // Ignore python version constraints
        if name == "python_version" || name == "python_full_version" {
            return Ok(None);
        }

        let dep_obj = value.as_object().ok_or_else(|| {
            crate::error::Error::DependencyParsing(format!(
                "Invalid dependency format for '{}': expected object",
                name
            ))
        })?;

        // Handle git dependencies
        if dep_obj.contains_key("git") {
            return self.parse_git_dependency(name, dep_obj, dep_type);
        }

        // Handle standard dependencies
        let version = match dep_obj.get("version") {
            Some(version_value) => {
                let version_str = version_value.as_str().ok_or_else(|| {
                    crate::error::Error::DependencyParsing(format!(
                        "Invalid version format for '{}': expected string",
                        name
                    ))
                })?;
                Some(self.clean_version(version_str))
            }
            None => None,
        };

        // Handle platform-specific dependencies
        let markers = self.extract_markers(dep_obj)?;

        Ok(Some(Dependency {
            name: name.to_string(),
            version,
            dep_type,
            environment_markers: markers,
            extras: None,
        }))
    }

    fn parse_git_dependency(
        &self,
        name: &str,
        dep_obj: &serde_json::Map<String, Value>,
        dep_type: DependencyType,
    ) -> Result<Option<Dependency>> {
        let git_url = dep_obj.get("git").and_then(|v| v.as_str()).ok_or_else(|| {
            crate::error::Error::DependencyParsing(format!("Invalid git URL for '{}'", name))
        })?;

        let ref_value = dep_obj.get("ref").and_then(|v| v.as_str());

        // Construct version string for git dependency
        let version = if let Some(git_ref) = ref_value {
            Some(format!("git+{}@{}", git_url, git_ref))
        } else {
            Some(format!("git+{}", git_url))
        };

        let markers = self.extract_markers(dep_obj)?;

        Ok(Some(Dependency {
            name: name.to_string(),
            version,
            dep_type,
            environment_markers: markers,
            extras: None,
        }))
    }

    fn extract_markers(&self, dep_obj: &serde_json::Map<String, Value>) -> Result<Option<String>> {
        let markers = match (
            dep_obj.get("markers"),
            dep_obj.get("platform_python_implementation"),
            dep_obj.get("platform"),
            dep_obj.get("sys_platform"),
        ) {
            (Some(markers), _, _, _) => Some(
                markers
                    .as_str()
                    .ok_or_else(|| {
                        crate::error::Error::DependencyParsing(
                            "Invalid markers format: expected string".to_string(),
                        )
                    })?
                    .to_string(),
            ),
            (_, Some(impl_value), _, _) => Some(format!(
                "platform_python_implementation == '{}'",
                impl_value
                    .as_str()
                    .ok_or_else(|| crate::error::Error::DependencyParsing(
                        "Invalid platform_python_implementation format".to_string()
                    ))?
            )),
            (_, _, Some(platform), _) => Some(format!(
                "platform == '{}'",
                platform
                    .as_str()
                    .ok_or_else(|| crate::error::Error::DependencyParsing(
                        "Invalid platform format".to_string()
                    ))?
            )),
            (_, _, _, Some(sys_platform)) => Some(format!(
                "sys_platform == '{}'",
                sys_platform
                    .as_str()
                    .ok_or_else(|| crate::error::Error::DependencyParsing(
                        "Invalid sys_platform format".to_string()
                    ))?
            )),
            _ => None,
        };

        Ok(markers)
    }

    fn clean_version(&self, version: &str) -> String {
        let version = version.trim();
        // Keep == intact, only clean single =
        if version.starts_with("==") {
            version.to_string()
        } else if let Some(stripped) = version.strip_prefix('=') {
            stripped.trim().to_string()
        } else {
            version.to_string()
        }
    }
}

impl MigrationSource for PipenvMigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>> {
        info!("Extracting dependencies from Pipfile.lock");
        let pipfile_lock_path = project_dir.join("Pipfile.lock");

        if !pipfile_lock_path.exists() {
            return Err(crate::error::Error::FileOperation {
                path: pipfile_lock_path.clone(),
                message: "Pipfile.lock does not exist".to_string(),
            });
        }

        let content = fs::read_to_string(&pipfile_lock_path).map_err(|e| {
            crate::error::Error::FileOperation {
                path: pipfile_lock_path.clone(),
                message: format!("Error reading file: {}", e),
            }
        })?;

        let lock_data: Value = serde_json::from_str(&content).map_err(|e| {
            crate::error::Error::DependencyParsing(format!("Error parsing Pipfile.lock: {}", e))
        })?;

        let mut dependencies = Vec::new();

        // Process default dependencies
        if let Some(default_deps) = lock_data.get("default").and_then(|v| v.as_object()) {
            debug!("Processing default dependencies");
            for (name, value) in default_deps {
                if let Some(dep) = self.parse_dependency(name, value, DependencyType::Main)? {
                    dependencies.push(dep);
                }
            }
        }

        // Process development dependencies
        if let Some(dev_deps) = lock_data.get("develop").and_then(|v| v.as_object()) {
            debug!("Processing development dependencies");
            for (name, value) in dev_deps {
                if let Some(dep) = self.parse_dependency(name, value, DependencyType::Dev)? {
                    dependencies.push(dep);
                }
            }
        }

        Ok(dependencies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};
    use tempfile::TempDir;

    fn create_test_pipfile_lock(content: &str) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();
        let pipfile_lock = project_dir.join("Pipfile.lock");
        fs::write(&pipfile_lock, content).unwrap();
        (temp_dir, project_dir)
    }

    #[test]
    fn test_complex_dependencies() {
        let content = r#"{
            "default": {
                "requests": {
                    "version": "==2.31.0",
                    "markers": "python_version >= '3.7'"
                },
                "flask": {
                    "version": ">=2.0.0,<3.0.0"
                }
            },
            "develop": {
                "pytest": {
                    "version": "==7.0.0"
                }
            }
        }"#;

        let (_temp_dir, project_dir) = create_test_pipfile_lock(content);
        let source = PipenvMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        assert_eq!(dependencies.len(), 3);

        let requests = dependencies.iter().find(|d| d.name == "requests").unwrap();
        assert_eq!(requests.version, Some("==2.31.0".to_string()));
        assert_eq!(
            requests.environment_markers,
            Some("python_version >= '3.7'".to_string())
        );

        let flask = dependencies.iter().find(|d| d.name == "flask").unwrap();
        assert_eq!(flask.version, Some(">=2.0.0,<3.0.0".to_string()));

        let pytest = dependencies.iter().find(|d| d.name == "pytest").unwrap();
        assert_eq!(pytest.version, Some("==7.0.0".to_string()));
        assert!(matches!(pytest.dep_type, DependencyType::Dev));
    }

    #[test]
    fn test_platform_specific_dependencies() {
        let content = r#"{
            "default": {
                "pywin32": {
                    "version": "==305",
                    "sys_platform": "win32"
                },
                "psutil": {
                    "version": "==5.9.0",
                    "platform": "linux"
                },
                "cpython": {
                    "version": "==0.0.1",
                    "platform_python_implementation": "CPython"
                }
            }
        }"#;

        let (_temp_dir, project_dir) = create_test_pipfile_lock(content);
        let source = PipenvMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        assert_eq!(dependencies.len(), 3);

        let pywin32 = dependencies.iter().find(|d| d.name == "pywin32").unwrap();
        assert_eq!(
            pywin32.environment_markers,
            Some("sys_platform == 'win32'".to_string())
        );

        let psutil = dependencies.iter().find(|d| d.name == "psutil").unwrap();
        assert_eq!(
            psutil.environment_markers,
            Some("platform == 'linux'".to_string())
        );

        let cpython = dependencies.iter().find(|d| d.name == "cpython").unwrap();
        assert_eq!(
            cpython.environment_markers,
            Some("platform_python_implementation == 'CPython'".to_string())
        );
    }

    #[test]
    fn test_git_dependencies() {
        let content = r#"{
            "default": {
                "custom-package": {
                    "git": "https://github.com/user/repo.git",
                    "ref": "master"
                },
                "another-package": {
                    "git": "https://github.com/user/another-repo.git"
                }
            }
        }"#;

        let (_temp_dir, project_dir) = create_test_pipfile_lock(content);
        let source = PipenvMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        assert_eq!(dependencies.len(), 2);

        let custom_pkg = dependencies
            .iter()
            .find(|d| d.name == "custom-package")
            .unwrap();
        assert_eq!(
            custom_pkg.version,
            Some("git+https://github.com/user/repo.git@master".to_string())
        );

        let another_pkg = dependencies
            .iter()
            .find(|d| d.name == "another-package")
            .unwrap();
        assert_eq!(
            another_pkg.version,
            Some("git+https://github.com/user/another-repo.git".to_string())
        );
    }

    #[test]
    fn test_ignore_scripts_section() {
        let content = r#"{
            "default": {
                "requests": {
                    "version": "==2.31.0"
                }
            },
            "scripts": {
                "test": "pytest"
            }
        }"#;

        let (_temp_dir, project_dir) = create_test_pipfile_lock(content);
        let source = PipenvMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0].name, "requests");
    }
}
