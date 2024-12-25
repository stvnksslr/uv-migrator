use std::fs;
use std::path::Path;
use log::debug;

/// Clean and validate a version string
fn clean_version(version: &str) -> Option<String> {
    let mut cleaned = version.trim().to_string();
    let mut prev_len;
    
    // Keep cleaning until no more changes occur
    loop {
        prev_len = cleaned.len();
        cleaned = cleaned
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .trim_matches(',')
            .trim()
            .to_string();
            
        if cleaned.len() == prev_len {
            break;
        }
    }
    
    // Basic version validation - should contain at least one number
    if cleaned.chars().any(|c| c.is_ascii_digit()) {
        Some(cleaned)
    } else {
        None
    }
}

/// Extracts the version from setup.py, __init__.py, or **version** file
///
/// # Arguments
///
/// * `project_dir` - The project directory to search for version information
///
/// # Returns
///
/// * `Result<Option<String>, String>` - The version if found, None if not found, or an error
pub fn extract_version(project_dir: &Path) -> Result<Option<String>, String> {
    // First try to get version from setup.py
    if let Some(version) = extract_version_from_setup_py(project_dir)? {
        debug!("Found version in setup.py: {}", version);
        return Ok(Some(version));
    }

    // Then try __init__.py files
    if let Some(version) = extract_version_from_init_py(project_dir)? {
        debug!("Found version in __init__.py: {}", version);
        return Ok(Some(version));
    }

    // Finally, try **version** file
    if let Some(version) = extract_version_from_version_file(project_dir)? {
        debug!("Found version in **version** file: {}", version);
        return Ok(Some(version));
    }

    Ok(None)
}

/// Extracts version from setup.py file
fn extract_version_from_setup_py(project_dir: &Path) -> Result<Option<String>, String> {
    let setup_py_path = project_dir.join("setup.py");
    if !setup_py_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&setup_py_path)
        .map_err(|e| format!("Failed to read setup.py: {}", e))?;

    // Look for version in setup() call
    if let Some(start_idx) = content.find("setup(") {
        let bracket_content = crate::migrators::setup_py::SetupPyMigrationSource::extract_setup_content(&content[start_idx..])?;
        
        if let Some(version) = crate::migrators::setup_py::SetupPyMigrationSource::extract_parameter(&bracket_content, "version") {
            if let Some(cleaned_version) = clean_version(&version) {
                return Ok(Some(cleaned_version));
            }
        }
    }

    Ok(None)
}

/// Extracts version from __init__.py file(s)
fn extract_version_from_init_py(project_dir: &Path) -> Result<Option<String>, String> {
    // First, try the direct __init__.py in the project directory
    let init_path = project_dir.join("__init__.py");
    if let Some(version) = extract_version_from_init_file(&init_path)? {
        return Ok(Some(version));
    }

    // Then, look for package directories
    for entry in fs::read_dir(project_dir)
        .map_err(|e| format!("Failed to read project directory: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() && !path.file_name().map_or(true, |n| n.to_string_lossy().starts_with('.')) {
            let init_path = path.join("__init__.py");
            if let Some(version) = extract_version_from_init_file(&init_path)? {
                return Ok(Some(version));
            }
        }
    }

    Ok(None)
}

/// Extracts version from a specific __init__.py file
fn extract_version_from_init_file(init_path: &Path) -> Result<Option<String>, String> {
    if !init_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(init_path)
        .map_err(|e| format!("Failed to read {}: {}", init_path.display(), e))?;

    // Look for __version__ = "X.Y.Z" pattern
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("__version__") {
            // Split by comment character and take first part
            let parts: Vec<&str> = line.splitn(2, '#').collect();
            let version_part = parts[0].splitn(2, '=').collect::<Vec<&str>>();
            if version_part.len() == 2 {
                if let Some(cleaned_version) = clean_version(version_part[1]) {
                    return Ok(Some(cleaned_version));
                }
            }
        }
    }

    Ok(None)
}

/// Extracts version from **version** file
fn extract_version_from_version_file(project_dir: &Path) -> Result<Option<String>, String> {
    let version_path = project_dir.join("**version**");
    if !version_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&version_path)
        .map_err(|e| format!("Failed to read **version** file: {}", e))?;

    if let Some(cleaned_version) = clean_version(&content) {
        Ok(Some(cleaned_version))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn test_clean_version() {
        let test_cases = vec![
            ("1.2.3", Some("1.2.3")),
            ("\"1.2.3\"", Some("1.2.3")),
            ("'1.2.3'", Some("1.2.3")),
            ("1.2.3,", Some("1.2.3")),
            (" 1.2.3 ", Some("1.2.3")),
            ("\"1.2.3\",", Some("1.2.3")),
            ("'1.2.3',", Some("1.2.3")),
            (" \"1.2.3\", ", Some("1.2.3")),
            (" '1.2.3', ", Some("1.2.3")),
            ("__version__,", None),
            ("", None),
            ("\"\"", None),
            ("\",\"", None),
            ("version", None),
        ];

        for (input, expected) in test_cases {
            assert_eq!(
                clean_version(input),
                expected.map(String::from),
                "Failed for input: {:?}", input
            );
        }
    }

    #[test]
    fn test_extract_version_from_init_py() {
        let temp_dir = create_test_dir();
        let pkg_dir = temp_dir.path().join("my_package");
        fs::create_dir(&pkg_dir).unwrap();
        
        let init_content = r#"
from .core import something

__version__ = "1.2.0"

def setup():
    pass
"#;
        fs::write(pkg_dir.join("__init__.py"), init_content).unwrap();

        let version = extract_version(&temp_dir.path()).unwrap();
        assert_eq!(version, Some("1.2.0".to_string()));
    }

    #[test]
    fn test_extract_version_from_init_py_single_quotes() {
        let temp_dir = create_test_dir();
        let pkg_dir = temp_dir.path().join("my_package");
        fs::create_dir(&pkg_dir).unwrap();
        
        let init_content = "__version__ = '1.2.0'";
        fs::write(pkg_dir.join("__init__.py"), init_content).unwrap();

        let version = extract_version(&temp_dir.path()).unwrap();
        assert_eq!(version, Some("1.2.0".to_string()));
    }

    #[test]
    fn test_extract_version_with_multiple_sources() {
        let temp_dir = create_test_dir();
        
        // Create setup.py with version
        let setup_py_content = r#"
from setuptools import setup

setup(
    name="test",
    version="2.0.0",
    description="Test project"
)
"#;
        fs::write(temp_dir.path().join("setup.py"), setup_py_content).unwrap();

        // Create package with __init__.py
        let pkg_dir = temp_dir.path().join("my_package");
        fs::create_dir(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("__init__.py"), r#"__version__ = "1.2.0""#).unwrap();

        // Create **version** file
        fs::write(temp_dir.path().join("**version**"), "3.0.0\n").unwrap();

        // Should prefer setup.py version
        let version = extract_version(&temp_dir.path()).unwrap();
        assert_eq!(version, Some("2.0.0".to_string()));
    }

    #[test]
    fn test_extract_version_precedence() {
        let temp_dir = create_test_dir();
        let pkg_dir = temp_dir.path().join("my_package");
        fs::create_dir(&pkg_dir).unwrap();
        
        // Create only __init__.py and **version**
        fs::write(pkg_dir.join("__init__.py"), r#"__version__ = "1.2.0""#).unwrap();
        fs::write(temp_dir.path().join("**version**"), "3.0.0\n").unwrap();

        // Should prefer __init__.py version when setup.py is absent
        let version = extract_version(&temp_dir.path()).unwrap();
        assert_eq!(version, Some("1.2.0".to_string()));
    }

    #[test]
    fn test_extract_version_with_invalid_values() {
        let temp_dir = create_test_dir();
        let pkg_dir = temp_dir.path().join("my_package");
        fs::create_dir(&pkg_dir).unwrap();
        
        // Test with invalid version string
        fs::write(pkg_dir.join("__init__.py"), r#"__version__ = "__version__,""#).unwrap();

        let version = extract_version(&temp_dir.path()).unwrap();
        assert_eq!(version, None);
    }

    #[test]
    fn test_extract_version_with_comma() {
        let temp_dir = create_test_dir();
        let pkg_dir = temp_dir.path().join("my_package");
        fs::create_dir(&pkg_dir).unwrap();
        
        // Test various combinations of quotes, commas, and comments
        let test_cases = vec![
            r#"__version__ = "1.2.0","#,
            r#"__version__ = '1.2.0',"#,
            r#"__version__ = "1.2.0", "#,
            r#"__version__ = "1.2.0",  # Comment"#,
            r#"__version__ = "1.2.0" # Comment"#,
            r#"__version__ = '1.2.0'  # With spaces and comment"#,
            r#"__version__ = "1.2.0",# No space before comment"#,
        ];
        
        for test_case in test_cases {
            fs::write(pkg_dir.join("__init__.py"), test_case).unwrap();
            let version = extract_version(&temp_dir.path()).unwrap();
            assert_eq!(version, Some("1.2.0".to_string()), 
                "Failed for case: {}", test_case);
            fs::remove_file(pkg_dir.join("__init__.py")).unwrap();
        }
    }
}