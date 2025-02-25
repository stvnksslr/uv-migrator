use crate::error::Result;
use crate::migrators::pipenv::PipenvMigrationSource;
use crate::migrators::poetry::PoetryMigrationSource;
use crate::models::project::ProjectType;
use log::info;
use std::path::Path;

pub fn detect_project_type(project_dir: &Path) -> Result<ProjectType> {
    let pyproject_path = project_dir.join("pyproject.toml");
    if pyproject_path.exists() {
        // First, check the project section (Poetry 2.0 style)
        if let Ok(content) = std::fs::read_to_string(&pyproject_path) {
            if let Ok(pyproject) = toml::from_str::<toml::Value>(&content) {
                // Check for Poetry 2.0 project section
                if let Some(project) = pyproject.get("project") {
                    if project.get("dependencies").is_some() {
                        info!("Detected Poetry 2.0 project");

                        // Don't automatically assume it's a package; let PoetryMigrationSource determine that
                        let poetry_type = PoetryMigrationSource::detect_project_type(project_dir)?;
                        return Ok(ProjectType::Poetry(poetry_type));
                    }
                }
            }
        }

        // Then check for traditional Poetry section
        if has_poetry_section(&pyproject_path)? {
            info!("Detected Poetry project");
            let poetry_type = PoetryMigrationSource::detect_project_type(project_dir)?;
            return Ok(ProjectType::Poetry(poetry_type));
        }
    }

    if PipenvMigrationSource::detect_project_type(project_dir) {
        info!("Detected Pipenv project");
        return Ok(ProjectType::Pipenv);
    }

    let setup_py_path = project_dir.join("setup.py");
    if setup_py_path.exists() {
        info!("Detected setuptools project");
        return Ok(ProjectType::SetupPy);
    }

    let requirements_files = find_requirements_files(project_dir);
    if !requirements_files.is_empty() {
        info!("Detected project with requirements files");
        return Ok(ProjectType::Requirements);
    }

    Err(crate::error::Error::ProjectDetection("Unable to detect project type. Ensure you have either a pyproject.toml with a [tool.poetry] section or a [project] section, a Pipfile, a setup.py file, or requirements.txt file(s).".to_string()))
}

/// Parses the contents of a TOML file to check for Poetry configuration.
///
/// # Arguments
///
/// * `pyproject_path` - The file path of the TOML file being parsed.
///
/// # Returns
///
/// * `bool` - Whether the file contains a Poetry configuration
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed
fn has_poetry_section(pyproject_path: &Path) -> Result<bool> {
    let content = std::fs::read_to_string(pyproject_path).map_err(|e| {
        crate::error::Error::FileOperation {
            path: pyproject_path.to_path_buf(),
            message: format!("Error reading file: {}", e),
        }
    })?;

    let pyproject: toml::Value =
        toml::from_str(&content).map_err(crate::error::Error::TomlSerde)?;

    // Check for traditional Poetry section
    let has_tool_poetry = pyproject
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .is_some();

    // Check for Poetry 2.0 project section
    let has_project_section = pyproject
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .is_some();

    Ok(has_tool_poetry || has_project_section)
}

/// Finds all requirements files in a directory.
///
/// Searches the specified directory for files that start with "requirements"
/// (e.g., requirements.txt, requirements-dev.txt). This includes any file
/// with a "requirements" prefix, regardless of its suffix.
///
/// # Arguments
///
/// * `dir` - A reference to a Path pointing to the directory to search
///
/// # Returns
///
/// A Vec<PathBuf> containing paths to all found requirements files.
fn find_requirements_files(dir: &Path) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file()
                && path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("requirements")
            {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}
