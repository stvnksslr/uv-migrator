use super::{MigrationSource, Dependency, DependencyType};
use std::path::{Path, PathBuf};
use std::fs;
use log::info;

pub struct RequirementsMigrationSource;

impl MigrationSource for RequirementsMigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>, String> {
        let requirements_files = self.find_requirements_files(project_dir);

        if requirements_files.is_empty() {
            return Err("No requirements files found.".to_string());
        }

        let mut dependencies = Vec::new();

        for file_path in requirements_files {
            let dep_type = if file_path.file_name().unwrap().to_str().unwrap() == "requirements.txt" {
                DependencyType::Main
            } else {
                DependencyType::Dev
            };

            info!("Processing requirements file: {}", file_path.display());

            let contents = fs::read_to_string(&file_path)
                .map_err(|e| format!("Error reading file '{}': {}", file_path.display(), e))?;

            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                if let Some(dep) = self.parse_requirement(line, dep_type.clone()) {
                    dependencies.push(dep);
                }
            }
        }

        Ok(dependencies)
    }
}

impl RequirementsMigrationSource {
    fn find_requirements_files(&self, dir: &Path) -> Vec<PathBuf> {
        fs::read_dir(dir)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_file() && path.file_name().unwrap().to_str().unwrap().starts_with("requirements") {
                    Some(path)
                } else {
                    None
                }
            })
            .collect()
    }

    fn parse_requirement(&self, line: &str, dep_type: DependencyType) -> Option<Dependency> {
        let parts: Vec<&str> = line.split(&['=', '>', '<', '~', '!'][..]).collect();
        let name = parts[0].trim().to_string();

        if name == "python" {
            return None;
        }

        let version = if parts.len() > 1 {
            Some(parts[1..].join("").trim().to_string())
        } else {
            None
        };

        Some(Dependency {
            name,
            version,
            dep_type,
        })
    }
}