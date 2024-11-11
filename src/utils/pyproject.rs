use log::{info, warn};
use std::path::{Path};

pub fn update_pyproject_toml(project_dir: &Path, extra_urls: &[String]) -> Result<(), String> {
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