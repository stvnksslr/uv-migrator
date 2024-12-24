use super::requirements::RequirementsMigrationSource;
use super::{Dependency, DependencyType, MigrationSource};
use log::{debug, info};
use std::fs;
use std::path::Path;

pub struct SetupPyMigrationSource;

impl MigrationSource for SetupPyMigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>, String> {
        info!("Extracting dependencies from setup.py");
        let requirements_source = RequirementsMigrationSource;
        if requirements_source.has_requirements_files(project_dir) {
            info!("Found requirements files, using requirements parser");
            return requirements_source.extract_dependencies(project_dir);
        }

        info!("No requirements files found, parsing setup.py directly");
        self.parse_setup_py(project_dir)
    }
}

impl SetupPyMigrationSource {
    fn parse_setup_py(&self, project_dir: &Path) -> Result<Vec<Dependency>, String> {
        let setup_py_path = project_dir.join("setup.py");
        let content = fs::read_to_string(&setup_py_path)
            .map_err(|e| format!("Failed to read setup.py: {}", e))?;

        debug!("Parsing setup.py content");
        let mut dependencies = Vec::new();

        // Extract main dependencies
        if let Some(mut deps) = self.extract_install_requires(&content) {
            dependencies.append(&mut deps);
        }

        // Extract test dependencies
        if let Some(mut deps) = self.extract_tests_require(&content) {
            dependencies.append(&mut deps);
        }

        Ok(dependencies)
    }

    fn extract_install_requires(&self, content: &str) -> Option<Vec<Dependency>> {
        let start_idx = content.find("install_requires=[")?;
        let bracket_content =
            self.extract_bracket_content(content, start_idx + "install_requires=".len())?;

        Some(self.parse_dependencies(&bracket_content, DependencyType::Main))
    }

    fn extract_tests_require(&self, content: &str) -> Option<Vec<Dependency>> {
        let start_idx = content.find("tests_require=[")?;
        let bracket_content =
            self.extract_bracket_content(content, start_idx + "tests_require=".len())?;

        Some(self.parse_dependencies(&bracket_content, DependencyType::Dev))
    }

    fn extract_bracket_content(&self, content: &str, start_pos: usize) -> Option<String> {
        let content = &content[start_pos..];
        let bracket_start = content.find('[')?;
        let mut bracket_count = 1;
        let mut pos = bracket_start + 1;

        while bracket_count > 0 && pos < content.len() {
            match content.chars().nth(pos)? {
                '[' => bracket_count += 1,
                ']' => bracket_count -= 1,
                _ => {}
            }
            pos += 1;
        }

        if bracket_count == 0 {
            Some(content[bracket_start + 1..pos - 1].to_string())
        } else {
            None
        }
    }

    fn parse_dependencies(&self, content: &str, dep_type: DependencyType) -> Vec<Dependency> {
        let mut dependencies = Vec::new();

        for line in content.split(',') {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Remove quotes and extract package name and version
            let dep_str = line.trim_matches(|c| c == '\'' || c == '"');
            if let Some((name, version)) = self.parse_dependency_spec(dep_str) {
                dependencies.push(Dependency {
                    name,
                    version,
                    dep_type: dep_type.clone(),
                    environment_markers: None,
                });
            }
        }

        dependencies
    }

    fn parse_dependency_spec(&self, dep_str: &str) -> Option<(String, Option<String>)> {
        if dep_str.is_empty() || dep_str == "setuptools" {
            return None;
        }

        // Handle different package specification formats
        if dep_str.contains(">=") {
            let parts: Vec<&str> = dep_str.split(">=").collect();
            Some((
                parts[0].trim().to_string(),
                Some(format!(">={}", parts[1].trim())),
            ))
        } else if dep_str.contains("==") {
            let parts: Vec<&str> = dep_str.split("==").collect();
            Some((
                parts[0].trim().to_string(),
                Some(parts[1].trim().to_string()),
            ))
        } else if dep_str.contains('>') {
            let parts: Vec<&str> = dep_str.split('>').collect();
            Some((
                parts[0].trim().to_string(),
                Some(format!(">{}", parts[1].trim())),
            ))
        } else {
            Some((dep_str.trim().to_string(), None))
        }
    }

    pub(crate) fn extract_setup_content(content: &str) -> Result<String, String> {
        let lines = content.lines().enumerate().peekable();
        let mut setup_content = String::new();
        let mut in_setup = false;
        let mut paren_count = 0;

        for (_, line) in lines {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if !in_setup {
                if trimmed.starts_with("setup(") {
                    in_setup = true;
                    paren_count = 1;
                    // Extract everything after setup(
                    if let Some(content) = line.split("setup(").nth(1) {
                        setup_content.push_str(content);
                        setup_content.push('\n');
                    }
                }
            } else {
                // Count parentheses in the line
                for c in line.chars() {
                    match c {
                        '(' => paren_count += 1,
                        ')' => paren_count -= 1,
                        _ => {}
                    }
                }

                setup_content.push_str(line);
                setup_content.push('\n');

                if paren_count == 0 {
                    break;
                }
            }
        }

        if !in_setup {
            return Err("Could not find setup() call".to_string());
        }
        if paren_count > 0 {
            return Err("Could not find matching closing parenthesis for setup()".to_string());
        }

        Ok(setup_content)
    }

    pub fn extract_description(project_dir: &Path) -> Result<Option<String>, String> {
        let setup_py_path = project_dir.join("setup.py");
        if !setup_py_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&setup_py_path)
            .map_err(|e| format!("Failed to read setup.py: {}", e))?;

        // Look for description in setup() call
        if let Some(start_idx) = content.find("setup(") {
            let bracket_content = Self::extract_setup_content(&content[start_idx..])?;

            // First try to find long_description
            if let Some(desc) = Self::extract_parameter(&bracket_content, "long_description") {
                debug!("Found long_description in setup.py");
                return Ok(Some(desc));
            }

            // Fall back to regular description
            if let Some(desc) = Self::extract_parameter(&bracket_content, "description") {
                debug!("Found description in setup.py");
                return Ok(Some(desc));
            }
        }

        Ok(None)
    }

    pub(crate) fn extract_parameter(content: &str, param_name: &str) -> Option<String> {
        let param_pattern = format!("{} = ", param_name);
        let param_pattern2 = format!("{}=", param_name);

        let lines = content.lines().peekable();
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with(&param_pattern) || trimmed.starts_with(&param_pattern2) {
                // Direct string assignment
                if trimmed.contains('"') || trimmed.contains('\'') {
                    if let Some(desc) = Self::extract_string_value(trimmed) {
                        return Some(desc);
                    }
                }
                // Single string variable
                else {
                    // For this case, just take the description parameter at face value
                    if param_name == "description" {
                        if let Some(value) = trimmed.split('=').nth(1) {
                            return Some(value.trim().to_string());
                        }
                    }
                }
            }
        }
        None
    }

    pub fn extract_string_value(line: &str) -> Option<String> {
        let after_equals = line.split('=').nth(1)?.trim();

        // Handle different quote types
        let (quote_char, content) = match after_equals.chars().next()? {
            '\'' => ('\'', &after_equals[1..]),
            '"' => ('"', &after_equals[1..]),
            _ => return None,
        };

        // Find matching end quote
        let end_pos = content.find(quote_char)?;
        let value = content[..end_pos].to_string();

        Some(value)
    }

    pub fn extract_url(project_dir: &Path) -> Result<Option<String>, String> {
        let setup_py_path = project_dir.join("setup.py");
        if !setup_py_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&setup_py_path)
            .map_err(|e| format!("Failed to read setup.py: {}", e))?;

        if let Some(start_idx) = content.find("setup(") {
            let bracket_content = Self::extract_setup_content(&content[start_idx..])?;
            if let Some(url) = Self::extract_parameter(&bracket_content, "url") {
                debug!("Found URL in setup.py");
                return Ok(Some(url));
            }
        }

        Ok(None)
    }
}
