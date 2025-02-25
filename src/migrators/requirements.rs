use crate::error::Result;
use crate::migrators::MigrationSource;
use crate::models::dependency::{Dependency, DependencyType};
use log::{debug, info};
use std::fs;
use std::path::{Path, PathBuf};

pub struct RequirementsMigrationSource;

impl MigrationSource for RequirementsMigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>> {
        let requirements_files = self.find_requirements_files(project_dir);
        if requirements_files.is_empty() {
            return Err(crate::error::Error::ProjectDetection(
                "No requirements files found.".to_string(),
            ));
        }

        let mut dependencies = Vec::new();
        for (file_path, dep_type) in requirements_files {
            info!("Processing requirements file: {}", file_path.display());
            let deps = self.process_requirements_file(&file_path, dep_type)?;
            debug!("Extracted {} dependencies", deps.len());
            dependencies.extend(deps);
        }

        debug!("Total dependencies extracted: {}", dependencies.len());
        Ok(dependencies)
    }
}

impl RequirementsMigrationSource {
    pub(crate) fn find_requirements_files(&self, dir: &Path) -> Vec<(PathBuf, DependencyType)> {
        let mut requirements_files = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name == "requirements.txt" {
                            requirements_files.push((path.clone(), DependencyType::Main));
                            info!("Found main requirements file: {}", path.display());
                        } else if file_name.starts_with("requirements-")
                            && file_name.ends_with(".txt")
                        {
                            let group_name = file_name
                                .strip_prefix("requirements-")
                                .unwrap()
                                .strip_suffix(".txt")
                                .unwrap();
                            let dep_type = match group_name {
                                "dev" => DependencyType::Dev,
                                _ => DependencyType::Group(group_name.to_string()),
                            };
                            requirements_files.push((path.clone(), dep_type));
                            info!("Found {} requirements file: {}", group_name, path.display());
                        }
                    }
                }
            }
        }
        requirements_files
    }

    pub fn has_requirements_files(&self, dir: &Path) -> bool {
        !self.find_requirements_files(dir).is_empty()
    }

    fn process_requirements_file(
        &self,
        file_path: &Path,
        dep_type: DependencyType,
    ) -> Result<Vec<Dependency>> {
        let contents =
            fs::read_to_string(file_path).map_err(|e| crate::error::Error::FileOperation {
                path: file_path.to_path_buf(),
                message: format!("Error reading file: {}", e),
            })?;

        let mut dependencies = Vec::new();

        for (line_num, line) in contents.lines().enumerate() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            match self.parse_requirement(line) {
                Ok(Some(dep)) => {
                    debug!("Parsed dependency on line {}: {:?}", line_num + 1, dep);
                    dependencies.push(Dependency {
                        name: dep.name,
                        version: dep.version,
                        dep_type: dep_type.clone(),
                        environment_markers: dep.environment_markers,
                        extras: dep.extras,
                    });
                }
                Ok(None) => debug!(
                    "Skipped line {} (possibly 'python' requirement): {}",
                    line_num + 1,
                    line
                ),
                Err(e) => debug!("Failed to parse line {}: {}", line_num + 1, e),
            }
        }

        debug!("Processed {} dependencies", dependencies.len());
        Ok(dependencies)
    }

    fn process_version_spec(&self, version_spec: &str) -> String {
        let version_spec = version_spec.trim();

        // For version specs with multiple constraints, preserve as-is
        if version_spec.contains(',') {
            return version_spec.to_string();
        }

        // Handle special cases in order of precedence
        if version_spec.starts_with("~=")
            || version_spec.starts_with(">=")
            || version_spec.starts_with("<=")
            || version_spec.starts_with(">")
            || version_spec.starts_with("<")
            || version_spec.starts_with("!=")
        {
            // Preserve these operators as-is
            version_spec.to_string()
        } else if let Some(stripped) = version_spec.strip_prefix("==") {
            // Remove double equals for exact versions
            stripped.to_string()
        } else if let Some(stripped) = version_spec.strip_prefix('~') {
            // Convert single tilde to tilde-equals
            format!("~={}", stripped)
        } else {
            // If no operator is present, preserve as-is
            version_spec.to_string()
        }
    }

    fn parse_requirement(&self, line: &str) -> Result<Option<Dependency>> {
        // Handle editable installs (-e flag)
        let line = if line.starts_with("-e") {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() != 2 {
                return Err(crate::error::Error::DependencyParsing(
                    "Invalid editable install format".to_string(),
                ));
            }
            parts[1]
        } else {
            line
        };

        // Split the line into package specification and environment markers
        let parts: Vec<&str> = line.split(';').collect();
        let package_spec = parts[0].trim();

        // Handle malformed lines
        if package_spec.is_empty() || package_spec.contains("===") {
            return Err(crate::error::Error::DependencyParsing(
                "Malformed requirement line".to_string(),
            ));
        }

        // Handle URLs and git repositories
        let (name, version) =
            if package_spec.starts_with("git+") || package_spec.starts_with("http") {
                self.parse_url_requirement(package_spec)?
            } else {
                self.parse_regular_requirement(package_spec)?
            };

        if name == "python" {
            return Ok(None);
        }

        // Handle environment markers
        let environment_markers = if parts.len() > 1 {
            Some(parts[1..].join(";").trim().to_string())
        } else {
            None
        };

        Ok(Some(Dependency {
            name,
            version,
            dep_type: DependencyType::Main, // This will be overridden by the caller
            environment_markers,
            extras: None,
        }))
    }

    fn parse_url_requirement(&self, package_spec: &str) -> Result<(String, Option<String>)> {
        let name = if let Some(egg_part) = package_spec.split('#').last() {
            if egg_part.starts_with("egg=") {
                egg_part.trim_start_matches("egg=").to_string()
            } else if package_spec.ends_with(".whl") {
                package_spec
                    .split('/')
                    .last()
                    .and_then(|f| f.split('-').next())
                    .ok_or_else(|| {
                        crate::error::Error::DependencyParsing("Invalid wheel filename".to_string())
                    })?
                    .to_string()
            } else {
                return Err(crate::error::Error::DependencyParsing(
                    "Invalid URL format".to_string(),
                ));
            }
        } else {
            package_spec
                .split('/')
                .last()
                .and_then(|f| f.split('.').next())
                .ok_or_else(|| {
                    crate::error::Error::DependencyParsing("Invalid URL format".to_string())
                })?
                .to_string()
        };

        Ok((name, None))
    }

    fn parse_regular_requirement(&self, package_spec: &str) -> Result<(String, Option<String>)> {
        // Return early if no version specifier is present
        if !package_spec.contains(&['>', '<', '=', '~', '!'][..]) {
            return Ok((package_spec.to_string(), None));
        }

        let name_end = package_spec
            .find(|c| ['>', '<', '=', '~', '!'].contains(&c))
            .unwrap();
        let name = package_spec[..name_end].trim().to_string();
        let version_spec = package_spec[name_end..].trim();

        let version = Some(self.process_version_spec(version_spec));

        Ok((name, version))
    }
}
