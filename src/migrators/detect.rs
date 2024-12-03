use log::info;
use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum ProjectType {
    Poetry,
    Requirements,
}

pub fn detect_project_type(project_dir: &Path) -> Result<ProjectType, String> {
    let pyproject_path = project_dir.join("pyproject.toml");

    if pyproject_path.exists() && has_poetry_section(&pyproject_path)? {
        info!("Detected Poetry project");
        return Ok(ProjectType::Poetry);
    }

    let requirements_files = find_requirements_files(project_dir);
    if !requirements_files.is_empty() {
        info!("Detected project with requirements files");
        return Ok(ProjectType::Requirements);
    }

    Err("Unable to detect project type. Ensure you have either a pyproject.toml with a [tool.poetry] section or requirements.txt file(s).".to_string())
}

fn has_poetry_section(pyproject_path: &Path) -> Result<bool, String> {
    let contents = std::fs::read_to_string(pyproject_path)
        .map_err(|e| format!("Error reading file '{}': {}", pyproject_path.display(), e))?;

    let pyproject: crate::types::PyProject = toml::from_str(&contents).map_err(|e| {
        format!(
            "Error parsing TOML in '{}': {}",
            pyproject_path.display(),
            e
        )
    })?;

    Ok(pyproject.tool.and_then(|t| t.poetry).is_some())
}

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
