use super::{MigrationSource, Dependency, DependencyType};
use std::path::{Path, PathBuf};
use std::fs;
use log::{info, debug};

pub struct RequirementsMigrationSource;

impl MigrationSource for RequirementsMigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>, String> {
        let (main_requirements, dev_requirements) = self.find_requirements_files(project_dir);

        if main_requirements.is_none() && dev_requirements.is_empty() {
            return Err("No requirements files found.".to_string());
        }

        let mut dependencies = Vec::new();

        // Process main requirements (requirements.txt)
        if let Some(main_file) = main_requirements {
            info!("Processing main requirements file: {}", main_file.display());
            let main_deps = self.process_requirements_file(&main_file, DependencyType::Main)?;
            debug!("Extracted {} main dependencies", main_deps.len());
            dependencies.extend(main_deps);
        }

        // Process dev requirements (requirements-*.txt)
        for dev_file in dev_requirements {
            info!("Processing dev requirements file: {}", dev_file.display());
            let dev_deps = self.process_requirements_file(&dev_file, DependencyType::Dev)?;
            debug!("Extracted {} dev dependencies from {}", dev_deps.len(), dev_file.display());
            dependencies.extend(dev_deps);
        }

        debug!("Total dependencies extracted: {}", dependencies.len());
        Ok(dependencies)
    }
}

impl RequirementsMigrationSource {
    fn find_requirements_files(&self, dir: &Path) -> (Option<PathBuf>, Vec<PathBuf>) {
        let mut main_requirements = None;
        let mut dev_requirements = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name == "requirements.txt" {
                            main_requirements = Some(path.clone());
                            info!("Found main requirements file: {}", path.display());
                        } else if file_name.starts_with("requirements-") && file_name.ends_with(".txt") {
                            dev_requirements.push(path.clone());
                            info!("Found dev requirements file: {}", path.display());
                        }
                    }
                }
            }
        }

        (main_requirements, dev_requirements)
    }

    fn process_requirements_file(&self, file_path: &Path, dep_type: DependencyType) -> Result<Vec<Dependency>, String> {
        let contents = fs::read_to_string(file_path)
            .map_err(|e| format!("Error reading file '{}': {}", file_path.display(), e))?;

        let mut dependencies = Vec::new();

        for (line_num, line) in contents.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            match self.parse_requirement(line, dep_type.clone()) {
                Some(dep) => {
                    debug!("Parsed dependency on line {}: {:?}", line_num + 1, dep);
                    dependencies.push(dep);
                },
                None => debug!("Skipped line {} (possibly 'python' requirement): {}", line_num + 1, line),
            }
        }

        debug!("Processed {} with {} dependencies", file_path.display(), dependencies.len());
        Ok(dependencies)
    }

    fn parse_requirement(&self, line: &str, dep_type: DependencyType) -> Option<Dependency> {
        // Split the line into package specification and environment markers
        let parts: Vec<&str> = line.split(';').collect();
        let package_spec = parts[0].trim();

        // Parse package name and version
        let pkg_parts: Vec<&str> = package_spec.split(&['=', '>', '<', '~', '!'][..]).collect();
        let name = pkg_parts[0].trim().to_string();

        if name == "python" {
            return None;
        }

        let version = if pkg_parts.len() > 1 {
            Some(pkg_parts[1..].join("").trim().to_string())
        } else {
            None
        };

        // Handle environment markers
        let environment_markers = if parts.len() > 1 {
            Some(parts[1..].join(";").trim().to_string())
        } else {
            None
        };

        Some(Dependency {
            name,
            version,
            dep_type,
            environment_markers,
        })
    }
}