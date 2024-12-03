use semver::Version;

pub fn check_uv_requirements() -> Result<(), String> {
    // First check if uv is in PATH
    let uv_path = match which::which("uv") {
        Ok(path) => path,
        Err(e) => {
            return Err(format!(
                "The 'uv' command is not available. Please install uv and ensure it's in your PATH. Error: {}",
                e
            ));
        }
    };

    // If uv is found, check its version
    let output = std::process::Command::new(&uv_path)
        .arg("--version")
        .output()
        .map_err(|e| format!("Failed to execute uv --version: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to get uv version: {}", stderr));
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    let version_str = version_output
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| "Unexpected uv version format".to_string())?;

    let current_version = Version::parse(version_str)
        .map_err(|e| format!("Failed to parse uv version '{}': {}", version_str, e))?;

    let min_version = Version::new(0, 5, 0);

    if current_version < min_version {
        return Err(format!(
            "uv version 0.5.0 or higher is required. Found version {}",
            current_version
        ));
    }

    Ok(())
}
