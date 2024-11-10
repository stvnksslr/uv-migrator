use log::info;
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