use crate::error::Result;
use crate::migrators::MigrationSource;
use crate::models::dependency::{Dependency, DependencyType};
use log::{debug, info};
use serde_json::Value;
use std::{fs, path::Path};

pub struct PipenvMigrationSource;

impl PipenvMigrationSource {
    pub fn detect_project_type(project_dir: &Path) -> bool {
        // Check for both Pipfile and Pipfile.lock
        let pipfile_path = project_dir.join("Pipfile");
        let pipfile_lock_path = project_dir.join("Pipfile.lock");

        pipfile_path.exists() && pipfile_lock_path.exists()
    }

    /// Parse a single dependency based on the Pipfile specification
    fn parse_pipfile_dependency(
        &self,
        name: &str,
        spec: &str,
        dep_type: DependencyType,
    ) -> Option<Dependency> {
        // Skip packages like 'python_version'
        if name == "python_version" {
            return None;
        }

        // Basic dependency parsing
        let version = match spec {
            "*" => None,
            spec if spec.starts_with('*') => None,
            spec => Some(spec.to_string()),
        };

        Some(Dependency {
            name: name.to_string(),
            version,
            dep_type,
            environment_markers: None,
            extras: None,
        })
    }

    /// Read and parse the Pipfile to determine dependencies
    fn read_pipfile(&self, project_dir: &Path) -> Result<Vec<Dependency>> {
        let pipfile_path = project_dir.join("Pipfile");
        let content =
            fs::read_to_string(&pipfile_path).map_err(|e| crate::error::Error::FileOperation {
                path: pipfile_path.clone(),
                message: format!("Error reading Pipfile: {}", e),
            })?;

        // Use toml crate to parse Pipfile
        let pipfile: toml::Value = toml::from_str(&content).map_err(|e| {
            crate::error::Error::DependencyParsing(format!("Error parsing Pipfile: {}", e))
        })?;

        let mut dependencies = Vec::new();

        // Parse main packages
        if let Some(packages) = pipfile.get("packages").and_then(|p| p.as_table()) {
            for (name, spec) in packages.iter() {
                let spec_str = match spec {
                    toml::Value::String(s) => s.as_str(),
                    _ => continue,
                };

                if let Some(dep) =
                    self.parse_pipfile_dependency(name, spec_str, DependencyType::Main)
                {
                    dependencies.push(dep);
                }
            }
        }

        // Parse dev packages
        if let Some(dev_packages) = pipfile.get("dev-packages").and_then(|p| p.as_table()) {
            for (name, spec) in dev_packages.iter() {
                let spec_str = match spec {
                    toml::Value::String(s) => s.as_str(),
                    _ => continue,
                };

                if let Some(dep) =
                    self.parse_pipfile_dependency(name, spec_str, DependencyType::Dev)
                {
                    dependencies.push(dep);
                }
            }
        }

        Ok(dependencies)
    }
}

impl MigrationSource for PipenvMigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>> {
        info!("Extracting dependencies from Pipfile");

        // First, read dependencies from Pipfile
        let pipfile_dependencies = self.read_pipfile(project_dir)?;

        // If no dependencies found in Pipfile, fallback to Pipfile.lock
        if pipfile_dependencies.is_empty() {
            self.extract_dependencies_from_lock_file(project_dir)
        } else {
            Ok(pipfile_dependencies)
        }
    }
}

/// Implementation for reading from Pipfile.lock (kept mostly the same as before)
impl PipenvMigrationSource {
    fn extract_dependencies_from_lock_file(&self, project_dir: &Path) -> Result<Vec<Dependency>> {
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

    // Existing parse_dependency method from the previous implementation
    fn parse_dependency(
        &self,
        name: &str,
        value: &Value,
        dep_type: DependencyType,
    ) -> Result<Option<Dependency>> {
        // Reuse the existing implementation from the previous code
        // (This method handles parsing from Pipfile.lock with complex dependency formats)
        // ... (keep the existing parse_dependency implementation)
        // Simplified version for this example
        if name == "python_version" || name == "python_full_version" {
            return Ok(None);
        }

        let dep_obj = value.as_object().ok_or_else(|| {
            crate::error::Error::DependencyParsing(format!(
                "Invalid dependency format for '{}': expected object",
                name
            ))
        })?;

        // Handle version specification
        let version = match dep_obj.get("version") {
            Some(version_value) => {
                let version_str = version_value.as_str().ok_or_else(|| {
                    crate::error::Error::DependencyParsing(format!(
                        "Invalid version format for '{}': expected string",
                        name
                    ))
                })?;
                Some(version_str.trim_start_matches('=').to_string())
            }
            None => None,
        };

        // Extract environment markers (optional)
        let markers = match (
            dep_obj.get("markers"),
            dep_obj.get("platform_python_implementation"),
            dep_obj.get("sys_platform"),
        ) {
            (Some(marker_val), _, _) => marker_val.as_str().map(|s| s.to_string()),
            (_, Some(impl_val), _) => impl_val
                .as_str()
                .map(|v| format!("platform_python_implementation == '{}'", v)),
            (_, _, Some(platform_val)) => platform_val
                .as_str()
                .map(|v| format!("sys_platform == '{}'", v)),
            _ => None,
        };

        Ok(Some(Dependency {
            name: name.to_string(),
            version,
            dep_type,
            environment_markers: markers,
            extras: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_pipfile_and_lock(
        pipfile_content: &str,
        lock_content: &str,
    ) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();

        fs::write(project_dir.join("Pipfile"), pipfile_content).unwrap();
        fs::write(project_dir.join("Pipfile.lock"), lock_content).unwrap();

        (temp_dir, project_dir)
    }

    #[test]
    fn test_pipfile_dependencies() {
        let pipfile_content = r#"
[packages]
fastapi = "*"
requests = "^2.31.0"

[dev-packages]
pytest = "^8.0.0"

[requires]
python_version = "3.12"
"#;

        let lock_content = r#"{
    "default": {
        "fastapi": {"version": "==0.111.0"},
        "requests": {"version": "==2.31.0"}
    },
    "develop": {
        "pytest": {"version": "==8.0.0"}
    }
}"#;

        let (_temp_dir, project_dir) = create_test_pipfile_and_lock(pipfile_content, lock_content);

        let source = PipenvMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        assert_eq!(dependencies.len(), 3);

        // Check Main dependencies
        let main_deps: Vec<_> = dependencies
            .iter()
            .filter(|d| matches!(d.dep_type, DependencyType::Main))
            .collect();
        assert_eq!(main_deps.len(), 2);

        let fastapi_dep = main_deps.iter().find(|d| d.name == "fastapi").unwrap();
        assert_eq!(
            fastapi_dep.version, None,
            "Fastapi should have no version from Pipfile"
        );

        let requests_dep = main_deps.iter().find(|d| d.name == "requests").unwrap();
        assert_eq!(requests_dep.version, Some("^2.31.0".to_string()));

        // Check Dev dependencies
        let dev_deps: Vec<_> = dependencies
            .iter()
            .filter(|d| matches!(d.dep_type, DependencyType::Dev))
            .collect();
        assert_eq!(dev_deps.len(), 1);

        let pytest_dep = dev_deps.iter().find(|d| d.name == "pytest").unwrap();
        assert_eq!(pytest_dep.version, Some("^8.0.0".to_string()));
    }

    #[test]
    fn test_pipfile_with_no_matching_lock_entries() {
        let pipfile_content = r#"
[packages]
custom-package = "*"

[dev-packages]
custom-dev-package = "^1.0.0"
"#;

        let lock_content = r#"{
    "default": {},
    "develop": {}
}"#;

        let (_temp_dir, project_dir) = create_test_pipfile_and_lock(pipfile_content, lock_content);

        let source = PipenvMigrationSource;
        let dependencies = source.extract_dependencies(&project_dir).unwrap();

        assert_eq!(dependencies.len(), 2);

        let main_deps: Vec<_> = dependencies
            .iter()
            .filter(|d| matches!(d.dep_type, DependencyType::Main))
            .collect();
        assert_eq!(main_deps.len(), 1);
        assert_eq!(main_deps[0].name, "custom-package");
        assert_eq!(main_deps[0].version, None);

        let dev_deps: Vec<_> = dependencies
            .iter()
            .filter(|d| matches!(d.dep_type, DependencyType::Dev))
            .collect();
        assert_eq!(dev_deps.len(), 1);
        assert_eq!(dev_deps[0].name, "custom-dev-package");
        assert_eq!(dev_deps[0].version, Some("^1.0.0".to_string()));
    }
}
