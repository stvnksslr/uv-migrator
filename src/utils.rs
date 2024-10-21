use log::{info, warn};
use std::process::Command;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

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

pub fn parse_pip_conf() -> Result<Vec<String>, String> {
    let home_dir = dirs::home_dir().ok_or_else(|| "Unable to determine home directory".to_string())?;
    let pip_conf_path = home_dir.join(".pip").join("pip.conf");

    if !pip_conf_path.exists() {
        return Ok(vec![]);
    }

    let file = File::open(&pip_conf_path)
        .map_err(|e| format!("Failed to open pip.conf: {}", e))?;
    let reader = BufReader::new(file);

    let mut extra_urls = vec![];
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Failed to read line from pip.conf: {}", e))?;
        let trimmed = line.trim();
        if trimmed.starts_with("extra-index-url") {
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() == 2 {
                extra_urls.push(parts[1].trim().to_string());
            }
        }
    }

    Ok(extra_urls)
}

pub fn update_pyproject_toml(project_dir: &PathBuf, extra_urls: &[String]) -> Result<(), String> {
    let pyproject_path = project_dir.join("pyproject.toml");
    let mut content = std::fs::read_to_string(&pyproject_path)
        .map_err(|e| format!("Failed to read pyproject.toml: {}", e))?;

    if !extra_urls.is_empty() {
        let uv_section = format!(
            "\n[tool.uv]\nextra-index-url = {}\n",
            serde_json::to_string(extra_urls).map_err(|e| format!("Failed to serialize extra URLs: {}", e))?
        );

        if content.contains("[tool.uv]") {
            warn!("[tool.uv] section already exists in pyproject.toml. Extra URLs might need manual merging.");
        } else {
            content.push_str(&uv_section);
        }

        std::fs::write(&pyproject_path, content)
            .map_err(|e| format!("Failed to write updated pyproject.toml: {}", e))?;

        info!("Updated pyproject.toml with extra index URLs");
    }

    Ok(())
}

