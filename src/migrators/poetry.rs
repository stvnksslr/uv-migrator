use crate::error::Error;
use crate::error::Result;
use crate::migrators::MigrationSource;
use crate::models::GitDependency;
use crate::models::dependency::{Dependency, DependencyType};
use crate::models::project::PoetryProjectType;
use crate::utils::toml::read_toml;
use log::{debug, info};
use std::fs;
use std::path::Path;
use toml_edit::{DocumentMut, Item, Value};

pub struct PoetryMigrationSource;

impl PoetryMigrationSource {
    pub fn detect_project_type(project_dir: &Path) -> Result<PoetryProjectType> {
        let pyproject_path = project_dir.join("pyproject.toml");
        let doc = read_toml(&pyproject_path)?;

        // First, check for actual package structure on disk - this is the strongest indicator
        if Self::verify_real_package_structure(project_dir) {
            debug!("Detected package structure on disk");
            return Ok(PoetryProjectType::Package);
        }

        // Check for explicit package configuration in pyproject.toml
        let has_explicit_packages = doc
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("packages"))
            .and_then(|p| p.as_array())
            .is_some_and(|pkgs| !pkgs.is_empty());

        if has_explicit_packages {
            debug!("Found explicit package configuration in pyproject.toml");
            return Ok(PoetryProjectType::Package);
        }

        // Check for setup.py, which is a strong indicator of a package
        if project_dir.join("setup.py").exists() {
            debug!("setup.py found, treating as package");
            return Ok(PoetryProjectType::Package);
        }

        // Check for project structure that indicates a package
        // Look for common Python package structures
        let project_name = extract_project_name_from_doc(&doc);
        if let Some(name) = &project_name {
            let snake_case_name = name.replace('-', "_").to_lowercase();

            // Check for common package directory patterns
            if project_dir
                .join(&snake_case_name)
                .join("__init__.py")
                .exists()
                || project_dir
                    .join("src")
                    .join(&snake_case_name)
                    .join("__init__.py")
                    .exists()
            {
                debug!("Found package structure matching project name");
                return Ok(PoetryProjectType::Package);
            }
        }

        // Having scripts alone doesn't make it a package - it depends on the project's intent
        // Look for other package indicators
        let has_package_indicators = Self::has_strong_package_indicators(&doc);

        if has_package_indicators {
            debug!("Strong package indicators found in pyproject.toml");
            return Ok(PoetryProjectType::Package);
        }

        // Default to application if no strong package indicators were found
        debug!("No strong package indicators found, defaulting to application");
        Ok(PoetryProjectType::Application)
    }

    /// Verifies if the project has a real package structure on disk
    pub fn verify_real_package_structure(project_dir: &Path) -> bool {
        // Check for "src" directory structure
        let src_dir = project_dir.join("src");
        if src_dir.exists() && src_dir.is_dir() {
            // Check if there are any Python modules (directories with __init__.py)
            // under the src directory
            if let Ok(entries) = std::fs::read_dir(&src_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && path.join("__init__.py").exists() {
                        return true;
                    }
                }
            }
        }

        // Check for package with the same name as the project directory
        if let Some(project_name) = project_dir.file_name().and_then(|name| name.to_str()) {
            let pkg_dir = project_dir.join(project_name);
            if pkg_dir.exists() && pkg_dir.is_dir() && pkg_dir.join("__init__.py").exists() {
                return true;
            }

            // Also try package name with underscores instead of dashes
            let pkg_name_underscores = project_name.replace('-', "_");
            if pkg_name_underscores != project_name {
                let pkg_dir = project_dir.join(&pkg_name_underscores);
                if pkg_dir.exists() && pkg_dir.is_dir() && pkg_dir.join("__init__.py").exists() {
                    return true;
                }
            }
        }

        false
    }

    /// Check for stronger package indicators beyond just having scripts
    fn has_strong_package_indicators(doc: &DocumentMut) -> bool {
        // Check for specific package-related configurations
        let has_classifiers = doc
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("classifiers"))
            .and_then(|c| c.as_array())
            .is_some_and(|classifiers| {
                classifiers.iter().any(|c| {
                    c.as_str().is_some_and(|s| {
                        s.contains("Development Status")
                            || s.contains("Programming Language")
                            || s.contains("License")
                    })
                })
            });

        let has_keywords = doc
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("keywords"))
            .and_then(|k| k.as_array())
            .is_some_and(|kw| !kw.is_empty());

        let has_readme = doc
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("readme"))
            .is_some();

        // Having multiple of these indicators suggests a more formal package
        (has_classifiers as u8) + (has_keywords as u8) + (has_readme as u8) >= 2
    }

    pub fn extract_python_version(project_dir: &Path) -> Result<Option<String>> {
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

    fn parse_poetry_v2_dep(&self, dep_str: &str) -> (String, Option<String>, Option<Vec<String>>) {
        // First, handle if there's a version constraint in parentheses
        let (base_dep, version) = if let Some(ver_idx) = dep_str.find('(') {
            let (base, ver_part) = dep_str.split_at(ver_idx);
            (
                base.trim().to_string(),
                Some(ver_part.trim_matches(&['(', ')'][..]).trim().to_string()),
            )
        } else {
            (dep_str.trim().to_string(), None)
        };

        // Then, extract extras if present
        let (name, extras) = if let Some(extras_start) = base_dep.find('[') {
            if let Some(extras_end) = base_dep.find(']') {
                let (name_part, extras_part) = base_dep.split_at(extras_start);
                let extras_str = &extras_part[1..extras_end - extras_start];
                let extras_vec = extras_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<_>>();

                (name_part.trim().to_string(), Some(extras_vec))
            } else {
                (base_dep, None)
            }
        } else {
            (base_dep, None)
        };

        (name, version, extras)
    }

    /// Extracts git dependencies from Poetry project
    pub fn extract_git_dependencies(&self, project_dir: &Path) -> Result<Vec<GitDependency>> {
        let old_pyproject_path = project_dir.join("old.pyproject.toml");
        if !old_pyproject_path.exists() {
            return Ok(Vec::new());
        }

        let content =
            fs::read_to_string(&old_pyproject_path).map_err(|e| Error::FileOperation {
                path: old_pyproject_path.clone(),
                message: format!("Failed to read old.pyproject.toml: {}", e),
            })?;

        let doc = content.parse::<DocumentMut>().map_err(Error::Toml)?;
        let mut git_dependencies = Vec::new();

        // Check tool.poetry.dependencies section (Poetry 1.0 style)
        if let Some(tool) = doc.get("tool") {
            if let Some(poetry) = tool.get("poetry") {
                if let Some(deps) = poetry.get("dependencies").and_then(|d| d.as_table()) {
                    for (name, value) in deps.iter() {
                        if let Some(git_dep) = self.extract_git_dependency_info(name, value) {
                            git_dependencies.push(git_dep);
                        }
                    }
                }

                // Also check group dependencies
                if let Some(groups) = poetry.get("group").and_then(|g| g.as_table()) {
                    for (_, group) in groups.iter() {
                        if let Some(deps) = group
                            .as_table()
                            .and_then(|g| g.get("dependencies"))
                            .and_then(|d| d.as_table())
                        {
                            for (name, value) in deps.iter() {
                                if let Some(git_dep) = self.extract_git_dependency_info(name, value)
                                {
                                    git_dependencies.push(git_dep);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Also check Poetry 2.0 style dependencies
        if let Some(project) = doc.get("project") {
            if let Some(_deps) = project.get("dependencies").and_then(|d| d.as_array()) {
                // For now, Poetry 2.0 git dependencies aren't handled
                // Would need additional parsing for the newer format
            }
        }

        Ok(git_dependencies)
    }

    /// Extracts git dependency information from a Poetry dependency definition
    fn extract_git_dependency_info(&self, name: &str, value: &Item) -> Option<GitDependency> {
        if name == "python" {
            return None;
        }

        match value {
            Item::Value(Value::InlineTable(table)) => {
                if let Some(git_url) = table.get("git").and_then(|v| v.as_str()) {
                    let mut git_dep = GitDependency {
                        name: name.to_string(),
                        git_url: git_url.to_string(),
                        branch: None,
                        tag: None,
                        rev: None,
                    };

                    // Extract branch, tag, or rev
                    if let Some(branch) = table.get("branch").and_then(|v| v.as_str()) {
                        git_dep.branch = Some(branch.to_string());
                    }

                    if let Some(tag) = table.get("tag").and_then(|v| v.as_str()) {
                        git_dep.tag = Some(tag.to_string());
                    }

                    if let Some(rev) = table.get("rev").and_then(|v| v.as_str()) {
                        git_dep.rev = Some(rev.to_string());
                    }

                    return Some(git_dep);
                }
            }
            Item::Table(table) => {
                if let Some(git_url) = table.get("git").and_then(|v| match v {
                    Item::Value(Value::String(s)) => Some(s.value()),
                    _ => None,
                }) {
                    let mut git_dep = GitDependency {
                        name: name.to_string(),
                        git_url: git_url.to_string(),
                        branch: None,
                        tag: None,
                        rev: None,
                    };

                    // Extract branch, tag, or rev
                    if let Some(branch) = table.get("branch").and_then(|v| match v {
                        Item::Value(Value::String(s)) => Some(s.value()),
                        _ => None,
                    }) {
                        git_dep.branch = Some(branch.to_string());
                    }

                    if let Some(tag) = table.get("tag").and_then(|v| match v {
                        Item::Value(Value::String(s)) => Some(s.value()),
                        _ => None,
                    }) {
                        git_dep.tag = Some(tag.to_string());
                    }

                    if let Some(rev) = table.get("rev").and_then(|v| match v {
                        Item::Value(Value::String(s)) => Some(s.value()),
                        _ => None,
                    }) {
                        git_dep.rev = Some(rev.to_string());
                    }

                    return Some(git_dep);
                }
            }
            _ => {}
        }

        None
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
                if v == "*" { None } else { Some(v.to_string()) }
            }
            Item::Value(Value::InlineTable(t)) => {
                let version_opt = t.get("version").and_then(|v| match v {
                    Value::String(s) => {
                        let version = s.value().trim();
                        if version == "*" {
                            None
                        } else {
                            Some(version.to_string())
                        }
                    }
                    _ => None,
                });

                // Store version to return later
                version_opt
            }
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

        // Extract extras if available
        let extras = match value {
            Item::Value(Value::InlineTable(t)) => t.get("extras").and_then(|e| match e {
                Value::Array(extras_array) => {
                    let extras = extras_array
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>();

                    if extras.is_empty() {
                        None
                    } else {
                        Some(extras)
                    }
                }
                _ => None,
            }),
            Item::Table(t) => t.get("extras").and_then(|e| match e {
                Item::Value(Value::Array(extras_array)) => {
                    let extras = extras_array
                        .iter()
                        .filter_map(|v| match v {
                            Value::String(s) => Some(s.value().to_string()),
                            _ => None,
                        })
                        .collect::<Vec<_>>();

                    if extras.is_empty() {
                        None
                    } else {
                        Some(extras)
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
            extras,
        })
    }
}

impl MigrationSource for PoetryMigrationSource {
    fn extract_dependencies(&self, project_dir: &Path) -> Result<Vec<Dependency>> {
        info!("Extracting dependencies from Poetry project");
        let pyproject_path = project_dir.join("pyproject.toml");

        if !pyproject_path.exists() {
            return Err(crate::error::Error::FileOperation {
                path: pyproject_path.clone(),
                message: format!("File does not exist: {}", pyproject_path.display()),
            });
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
                        // Split the dependency string into name, version, and extras
                        let (name, version, extras) = self.parse_poetry_v2_dep(dep_str);

                        let dep = Dependency {
                            name,
                            version,
                            dep_type: DependencyType::Main,
                            environment_markers: None,
                            extras,
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

// Helper function to extract project name from TOML document
fn extract_project_name_from_doc(doc: &DocumentMut) -> Option<String> {
    // Try project section first (Poetry 2.0)
    if let Some(name) = doc
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
    {
        return Some(name.to_string());
    }

    // Then try tool.poetry
    if let Some(name) = doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
    {
        return Some(name.to_string());
    }

    None
}
