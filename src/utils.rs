use log::info;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn create_virtual_environment() -> Result<(), String> {
    info!("Creating a new virtual environment");
    let uv_path = which::which("uv").map_err(|e| format!("Failed to find uv command: {}", e))?;
    let output = Command::new(uv_path).arg("venv").output().map_err(|e| {
        format!("Failed to execute uv venv: {}", e)
    })?;

    if output.status.success() {
        info!("Virtual environment created successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to create virtual environment: {}", stderr))
    }
}

pub fn format_dependency(name: &str, value: &toml::Value) -> String {
    match value {
        toml::Value::String(v) => format!("{}=={}", name, v.trim_start_matches('^')),
        toml::Value::Table(t) => {
            if let Some(toml::Value::String(version)) = t.get("version") {
                format!("{}=={}", name, version.trim_start_matches('^'))
            } else {
                name.to_string()
            }
        }
        _ => name.to_string(),
    }
}

pub fn should_include_dependency(dep: &str, formatted_dep: &str) -> bool {
    !(dep == "python" || formatted_dep.starts_with("python=="))
}

pub fn find_pyproject_toml(path: &Path) -> Option<PathBuf> {
    let pyproject_path = path.join("pyproject.toml");
    if pyproject_path.exists() {
        Some(pyproject_path)
    } else {
        None
    }
}