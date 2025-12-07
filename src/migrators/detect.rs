use crate::error::Result;
use crate::migrators::conda::CondaMigrationSource;
use crate::migrators::pipenv::PipenvMigrationSource;
use crate::migrators::poetry::PoetryMigrationSource;
use crate::models::project::ProjectType;
use log::info;
use std::path::Path;

pub fn detect_project_type(project_dir: &Path) -> Result<ProjectType> {
    // Check for Conda environment first (most specific)
    if CondaMigrationSource::detect_project_type(project_dir) {
        info!("Detected Conda project");
        return Ok(ProjectType::Conda);
    }

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

    Err(crate::error::Error::ProjectDetection("Unable to detect project type. Ensure you have either a pyproject.toml with a [tool.poetry] section or a [project] section, a Pipfile, a setup.py file, requirements.txt file(s), or an environment.yml file for Conda projects.".to_string()))
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
/// Returns an empty Vec if the directory cannot be read.
fn find_requirements_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            log::debug!("Could not read directory {}: {}", dir.display(), e);
            return Vec::new();
        }
    };

    entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name()?.to_str()?;
                if file_name.starts_with("requirements") {
                    return Some(path);
                }
            }
            None
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_temp_dir() -> TempDir {
        TempDir::new().expect("Failed to create temp directory")
    }

    #[test]
    fn test_detect_poetry_project() {
        let temp_dir = create_temp_dir();
        let project_dir = temp_dir.path();

        let pyproject_content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.9"
requests = "^2.28.0"
"#;
        fs::write(project_dir.join("pyproject.toml"), pyproject_content).unwrap();

        let result = detect_project_type(project_dir);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), ProjectType::Poetry(_)));
    }

    #[test]
    fn test_detect_poetry2_project() {
        let temp_dir = create_temp_dir();
        let project_dir = temp_dir.path();

        let pyproject_content = r#"
[project]
name = "test-project"
version = "0.1.0"
dependencies = ["requests>=2.28.0"]
"#;
        fs::write(project_dir.join("pyproject.toml"), pyproject_content).unwrap();

        let result = detect_project_type(project_dir);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), ProjectType::Poetry(_)));
    }

    #[test]
    fn test_detect_pipenv_project() {
        let temp_dir = create_temp_dir();
        let project_dir = temp_dir.path();

        let pipfile_content = r#"
[[source]]
url = "https://pypi.org/simple"
verify_ssl = true
name = "pypi"

[packages]
requests = "*"

[dev-packages]
pytest = "*"
"#;
        fs::write(project_dir.join("Pipfile"), pipfile_content).unwrap();
        fs::write(project_dir.join("Pipfile.lock"), "{}").unwrap();

        let result = detect_project_type(project_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ProjectType::Pipenv);
    }

    #[test]
    fn test_detect_setup_py_project() {
        let temp_dir = create_temp_dir();
        let project_dir = temp_dir.path();

        let setup_py_content = r#"
from setuptools import setup

setup(
    name="test-project",
    version="0.1.0",
    install_requires=["requests>=2.28.0"],
)
"#;
        fs::write(project_dir.join("setup.py"), setup_py_content).unwrap();

        let result = detect_project_type(project_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ProjectType::SetupPy);
    }

    #[test]
    fn test_detect_requirements_project() {
        let temp_dir = create_temp_dir();
        let project_dir = temp_dir.path();

        fs::write(project_dir.join("requirements.txt"), "requests>=2.28.0\n").unwrap();

        let result = detect_project_type(project_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ProjectType::Requirements);
    }

    #[test]
    fn test_detect_requirements_dev_project() {
        let temp_dir = create_temp_dir();
        let project_dir = temp_dir.path();

        fs::write(project_dir.join("requirements-dev.txt"), "pytest>=7.0.0\n").unwrap();

        let result = detect_project_type(project_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ProjectType::Requirements);
    }

    #[test]
    fn test_detect_conda_project() {
        let temp_dir = create_temp_dir();
        let project_dir = temp_dir.path();

        let env_content = r#"
name: test-env
dependencies:
  - python=3.9
  - numpy
"#;
        fs::write(project_dir.join("environment.yml"), env_content).unwrap();

        let result = detect_project_type(project_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ProjectType::Conda);
    }

    #[test]
    fn test_detect_fails_with_no_config() {
        let temp_dir = create_temp_dir();
        let project_dir = temp_dir.path();

        // Empty directory
        let result = detect_project_type(project_dir);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(matches!(error, crate::error::Error::ProjectDetection(_)));
    }

    #[test]
    fn test_detect_priority_conda_over_poetry() {
        let temp_dir = create_temp_dir();
        let project_dir = temp_dir.path();

        // Create both Conda and Poetry configs
        let env_content = r#"
name: test-env
dependencies:
  - python=3.9
"#;
        fs::write(project_dir.join("environment.yml"), env_content).unwrap();

        let pyproject_content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"
"#;
        fs::write(project_dir.join("pyproject.toml"), pyproject_content).unwrap();

        // Conda should take priority
        let result = detect_project_type(project_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ProjectType::Conda);
    }

    #[test]
    fn test_detect_priority_poetry_over_pipenv() {
        let temp_dir = create_temp_dir();
        let project_dir = temp_dir.path();

        // Create both Poetry and Pipenv configs
        let pyproject_content = r#"
[tool.poetry]
name = "test-project"
version = "0.1.0"
"#;
        fs::write(project_dir.join("pyproject.toml"), pyproject_content).unwrap();

        let pipfile_content = r#"
[packages]
requests = "*"
"#;
        fs::write(project_dir.join("Pipfile"), pipfile_content).unwrap();
        fs::write(project_dir.join("Pipfile.lock"), "{}").unwrap();

        // Poetry should take priority
        let result = detect_project_type(project_dir);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), ProjectType::Poetry(_)));
    }

    #[test]
    fn test_find_requirements_files() {
        let temp_dir = create_temp_dir();
        let project_dir = temp_dir.path();

        fs::write(project_dir.join("requirements.txt"), "").unwrap();
        fs::write(project_dir.join("requirements-dev.txt"), "").unwrap();
        fs::write(project_dir.join("requirements_test.txt"), "").unwrap();
        fs::write(project_dir.join("other.txt"), "").unwrap();

        let files = find_requirements_files(project_dir);
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_find_requirements_files_empty_dir() {
        let temp_dir = create_temp_dir();
        let project_dir = temp_dir.path();

        let files = find_requirements_files(project_dir);
        assert!(files.is_empty());
    }

    #[test]
    fn test_find_requirements_files_nonexistent_dir() {
        let nonexistent = Path::new("/nonexistent/path/that/does/not/exist");
        let files = find_requirements_files(nonexistent);
        assert!(files.is_empty()); // Should not panic, returns empty vec
    }
}
